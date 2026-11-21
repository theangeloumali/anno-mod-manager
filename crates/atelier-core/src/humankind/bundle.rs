//! Reads a module's `RuntimeModule` (type, author, description) from its UnityFS `.assetbundle`.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

const SIGNATURE: &[u8] = b"UnityFS\0";
const HEADER_PROBE: u64 = 1024;
const COMPRESSION_MASK: u32 = 0x3F;
const LZ4: u32 = 2;
const LZ4_HC: u32 = 3;
/// LZ4 never expands past 255:1, so a bigger claim is a corrupt header, not an allocation to make.
const LZ4_MAX_RATIO: u64 = 255;
const BLOCKS_INFO_AT_END: u32 = 0x80;
const PAD_BEFORE_DATA: u32 = 0x200;
const SERIALIZED_FILE_NODE: u32 = 4;
const BLOCK_ENTRY_SIZE: usize = 10;
const NODE_ENTRY_MIN: usize = 21;
const SERIALIZED_HEADER: u64 = 48;
const SERIALIZED_VERSION: u32 = 22;
const MONO_BEHAVIOUR: i32 = 114;
const TYPE_ENTRY_MIN: usize = 31;
const TYPE_NODE_SIZE: usize = 32;
const OBJECT_ENTRY_SIZE: usize = 24;
const ARRAY_FLAG: u8 = 1;
const ALIGN_FLAG: i32 = 0x4000;
const COMMON_STRING: u32 = 0x8000_0000;
/// Zero-size elements nested in arrays consume no bytes, so only a node budget bounds the walk.
const MAX_WALKED_NODES: usize = 1 << 20;
const MODULE_TYPE_FIELD: &str = "RuntimeModuleType";
const AUTHOR_FIELD: &str = "Author";
const DESCRIPTION_FIELD: &str = "Description";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleType {
    Standalone,
    Conversion,
    Extension,
}

impl ModuleType {
    /// The game's `Amplitude.Mercury.Runtime.RuntimeModuleType` values.
    pub fn from_raw(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Standalone),
            1 => Some(Self::Conversion),
            2 => Some(Self::Extension),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standalone => "Standalone",
            Self::Conversion => "Conversion",
            Self::Extension => "Extension",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInfo {
    pub module_type: ModuleType,
    pub author: String,
    pub description: String,
}

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("not a UnityFS bundle")]
    NotUnityFs,
    #[error("bundle is truncated or malformed")]
    Malformed,
    #[error("bundle compression {0} is not supported")]
    Compression(u32),
    #[error("serialized file version {0} is not supported")]
    SerializedVersion(u32),
    #[error("bundle was built without type trees")]
    NoTypeTree,
    #[error("bundle holds no RuntimeModule")]
    NoRuntimeModule,
}

type BundleResult<T> = Result<T, BundleError>;

/// Reads only the headers and the one object it needs, never the whole (often huge) bundle.
pub fn read_module_info(path: &Path) -> BundleResult<ModuleInfo> {
    let file = File::open(path)?;
    let len = file.metadata()?.len();
    let mut bundle = BundleFile { file, len };
    for (start, size) in serialized_files(&mut bundle)? {
        if let Some(info) = read_runtime_module(&mut bundle, start, size)? {
            return Ok(info);
        }
    }
    Err(BundleError::NoRuntimeModule)
}

struct BundleFile {
    file: File,
    len: u64,
}

impl BundleFile {
    fn read(&mut self, offset: u64, len: u64) -> BundleResult<Vec<u8>> {
        let end = offset.checked_add(len).ok_or(BundleError::Malformed)?;
        if end > self.len {
            return Err(BundleError::Malformed);
        }
        let mut buffer = vec![0; usize::try_from(len).map_err(|_| BundleError::Malformed)?];
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut buffer)?;
        Ok(buffer)
    }
}

/// Absolute `(start, size)` of each serialized file packed in the bundle.
fn serialized_files(bundle: &mut BundleFile) -> BundleResult<Vec<(u64, u64)>> {
    let head = bundle.read(0, bundle.len.min(HEADER_PROBE))?;
    if !head.starts_with(SIGNATURE) {
        return Err(BundleError::NotUnityFs);
    }
    let mut cursor = Cursor::new(&head, true);
    cursor.cstr()?;
    let format = cursor.u32()?;
    cursor.cstr()?;
    cursor.cstr()?;
    cursor.u64()?;
    let packed_size = u64::from(cursor.u32()?);
    let unpacked_size = u64::from(cursor.u32()?);
    let flags = cursor.u32()?;
    if format >= 7 {
        cursor.align(16)?;
    }
    let header_end = cursor.position as u64;
    let (info_offset, mut data_start) = if flags & BLOCKS_INFO_AT_END != 0 {
        let offset = bundle.len.checked_sub(packed_size);
        (offset.ok_or(BundleError::Malformed)?, header_end)
    } else {
        (header_end, header_end + packed_size)
    };
    if flags & PAD_BEFORE_DATA != 0 {
        data_start = data_start.next_multiple_of(16);
    }
    let packed = bundle.read(info_offset, packed_size)?;
    let info = unpack(packed, unpacked_size, flags & COMPRESSION_MASK)?;

    let mut cursor = Cursor::new(&info, true);
    cursor.skip(16)?;
    for _ in 0..cursor.count(BLOCK_ENTRY_SIZE)? {
        cursor.skip(8)?;
        // Compressed data blocks would need a full decompress; every bundle seen so far stores them raw.
        let compression = u32::from(cursor.u16()?) & COMPRESSION_MASK;
        if compression != 0 {
            return Err(BundleError::Compression(compression));
        }
    }
    let mut files = Vec::new();
    for _ in 0..cursor.count(NODE_ENTRY_MIN)? {
        let offset = cursor.u64()?;
        let size = cursor.u64()?;
        let node_flags = cursor.u32()?;
        cursor.cstr()?;
        if node_flags & SERIALIZED_FILE_NODE != 0 {
            let start = data_start.checked_add(offset);
            files.push((start.ok_or(BundleError::Malformed)?, size));
        }
    }
    Ok(files)
}

fn unpack(packed: Vec<u8>, unpacked_size: u64, compression: u32) -> BundleResult<Vec<u8>> {
    match compression {
        0 => Ok(packed),
        LZ4 | LZ4_HC => {
            if unpacked_size > packed.len() as u64 * LZ4_MAX_RATIO + 16 {
                return Err(BundleError::Malformed);
            }
            let size = usize::try_from(unpacked_size).map_err(|_| BundleError::Malformed)?;
            let unpacked =
                lz4_flex::block::decompress(&packed, size).map_err(|_| BundleError::Malformed)?;
            if unpacked.len() == size {
                Ok(unpacked)
            } else {
                Err(BundleError::Malformed)
            }
        }
        other => Err(BundleError::Compression(other)),
    }
}

/// `None` when this serialized file has no object of the RuntimeModule type.
fn read_runtime_module(
    bundle: &mut BundleFile,
    start: u64,
    size: u64,
) -> BundleResult<Option<ModuleInfo>> {
    if size < SERIALIZED_HEADER {
        return Err(BundleError::Malformed);
    }
    let header = bundle.read(start, SERIALIZED_HEADER)?;
    let mut cursor = Cursor::new(&header, true);
    cursor.skip(8)?;
    let version = cursor.u32()?;
    if version != SERIALIZED_VERSION {
        return Err(BundleError::SerializedVersion(version));
    }
    cursor.skip(4)?;
    let big_endian = cursor.u8()? != 0;
    cursor.skip(3)?;
    let metadata_size = u64::from(cursor.u32()?);
    cursor.u64()?;
    let data_offset = cursor.u64()?;
    if SERIALIZED_HEADER + metadata_size > size {
        return Err(BundleError::Malformed);
    }
    let metadata = bundle.read(start + SERIALIZED_HEADER, metadata_size)?;
    let Some((tree, object_start, object_size)) = find_module_object(&metadata, big_endian)? else {
        return Ok(None);
    };
    let object_offset = data_offset
        .checked_add(object_start)
        .filter(|offset| offset.saturating_add(object_size) <= size)
        .ok_or(BundleError::Malformed)?;
    let object_at = start.checked_add(object_offset);
    let object = bundle.read(object_at.ok_or(BundleError::Malformed)?, object_size)?;
    tree.read_module(&object, big_endian).map(Some)
}

/// The RuntimeModule type tree plus its object's `(byte start, byte size)` in the data area.
fn find_module_object(
    metadata: &[u8],
    big_endian: bool,
) -> BundleResult<Option<(TypeTree, u64, u64)>> {
    let mut cursor = Cursor::new(metadata, big_endian);
    cursor.cstr()?;
    cursor.skip(4)?;
    if cursor.u8()? == 0 {
        return Err(BundleError::NoTypeTree);
    }
    let mut module_type = None;
    for index in 0..cursor.count(TYPE_ENTRY_MIN)? {
        let tree = TypeTree::parse(&mut cursor)?;
        if module_type.is_none() && tree.is_runtime_module() {
            module_type = Some((index, tree));
        }
    }
    let Some((wanted, tree)) = module_type else {
        return Ok(None);
    };
    for _ in 0..cursor.count(OBJECT_ENTRY_SIZE)? {
        cursor.align(4)?;
        cursor.skip(8)?;
        let byte_start = cursor.u64()?;
        let byte_size = u64::from(cursor.u32()?);
        if usize::try_from(cursor.i32()?) == Ok(wanted) {
            return Ok(Some((tree, byte_start, byte_size)));
        }
    }
    Ok(None)
}

struct TypeNode {
    level: u8,
    flags: u8,
    byte_size: i32,
    meta_flags: i32,
    /// Only names from the type's own string buffer; Unity's shared names stay `None`.
    name: Option<String>,
}

struct TypeTree {
    nodes: Vec<TypeNode>,
    children: Vec<Vec<usize>>,
}

impl TypeTree {
    fn parse(cursor: &mut Cursor) -> BundleResult<Self> {
        let class_id = cursor.i32()?;
        cursor.skip(3)?;
        cursor.skip(if class_id == MONO_BEHAVIOUR { 32 } else { 16 })?;
        let node_count = cursor.count(TYPE_NODE_SIZE)?;
        let strings_size = usize::try_from(cursor.i32()?).map_err(|_| BundleError::Malformed)?;
        let raw_nodes = cursor.take(node_count * TYPE_NODE_SIZE)?;
        let strings = cursor.take(strings_size)?;
        let dependency_count = cursor.count(4)?;
        cursor.skip(dependency_count * 4)?;

        let mut raw = Cursor::new(raw_nodes, cursor.big_endian);
        let mut nodes = Vec::with_capacity(node_count);
        for _ in 0..node_count {
            raw.skip(2)?;
            let level = raw.u8()?;
            let flags = raw.u8()?;
            raw.skip(4)?;
            let name_offset = raw.u32()?;
            let byte_size = raw.i32()?;
            raw.skip(4)?;
            let meta_flags = raw.i32()?;
            raw.skip(8)?;
            let name = (name_offset & COMMON_STRING == 0)
                .then(|| local_string(strings, name_offset))
                .flatten();
            nodes.push(TypeNode {
                level,
                flags,
                byte_size,
                meta_flags,
                name,
            });
        }
        let children = children_of(&nodes);
        Ok(Self { nodes, children })
    }

    fn root_fields(&self) -> &[usize] {
        self.children.first().map_or(&[], Vec::as_slice)
    }

    fn field_named(&self, name: &str) -> bool {
        let fields = self.root_fields().iter();
        fields
            .map(|&field| self.nodes[field].name.as_deref())
            .any(|field| field == Some(name))
    }

    fn is_runtime_module(&self) -> bool {
        self.field_named(MODULE_TYPE_FIELD) && self.field_named(AUTHOR_FIELD)
    }

    /// Walks the top-level fields in order, keeping the three it needs.
    fn read_module(&self, object: &[u8], big_endian: bool) -> BundleResult<ModuleInfo> {
        let mut cursor = Cursor::new(object, big_endian);
        let (mut module_type, mut author, mut description) = (None, None, None);
        let mut budget = MAX_WALKED_NODES;
        for &field in self.root_fields() {
            let start = cursor.position;
            self.walk(field, &mut cursor, &mut budget)?;
            let bytes = object.get(start..cursor.position);
            let mut value = Cursor::new(bytes.ok_or(BundleError::Malformed)?, big_endian);
            match self.nodes[field].name.as_deref() {
                Some(MODULE_TYPE_FIELD) => {
                    let raw = value.i32()?;
                    module_type = Some(ModuleType::from_raw(raw).ok_or(BundleError::Malformed)?);
                }
                Some(AUTHOR_FIELD) => author = Some(value.string()?),
                Some(DESCRIPTION_FIELD) => description = Some(value.string()?),
                _ => {}
            }
            if let (Some(module_type), Some(author), Some(description)) =
                (module_type, &author, &description)
            {
                return Ok(ModuleInfo {
                    module_type,
                    author: author.clone(),
                    description: description.clone(),
                });
            }
        }
        Err(BundleError::Malformed)
    }

    /// Advances past one serialized value of node `index`, following Unity's array and align rules.
    fn walk(&self, index: usize, cursor: &mut Cursor, budget: &mut usize) -> BundleResult<()> {
        *budget = budget.checked_sub(1).ok_or(BundleError::Malformed)?;
        let node = &self.nodes[index];
        let children = &self.children[index];
        if node.flags & ARRAY_FLAG != 0 {
            let element = *children.get(1).ok_or(BundleError::Malformed)?;
            let count = cursor.count(1)?;
            let element_node = &self.nodes[element];
            let is_leaf = self.children[element].is_empty();
            if is_leaf && element_node.byte_size <= 0 && count > 0 {
                return Err(BundleError::Malformed);
            }
            let plain =
                is_leaf && element_node.byte_size > 0 && element_node.meta_flags & ALIGN_FLAG == 0;
            if plain {
                let element_size = element_node.byte_size as usize;
                cursor.skip(
                    count
                        .checked_mul(element_size)
                        .ok_or(BundleError::Malformed)?,
                )?;
            } else {
                for _ in 0..count {
                    self.walk(element, cursor, budget)?;
                }
            }
        } else if children.is_empty() {
            cursor.skip(usize::try_from(node.byte_size).map_err(|_| BundleError::Malformed)?)?;
        } else {
            for &child in children {
                self.walk(child, cursor, budget)?;
            }
        }
        if node.meta_flags & ALIGN_FLAG != 0 {
            cursor.align(4)?;
        }
        Ok(())
    }
}

/// Nodes are stored depth-first with a level each; children are the next level down.
fn children_of(nodes: &[TypeNode]) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); nodes.len()];
    let mut ancestors: Vec<usize> = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        while ancestors
            .last()
            .is_some_and(|&parent| nodes[parent].level >= node.level)
        {
            ancestors.pop();
        }
        if let Some(&parent) = ancestors.last() {
            children[parent].push(index);
        }
        ancestors.push(index);
    }
    children
}

fn local_string(strings: &[u8], offset: u32) -> Option<String> {
    let rest = strings.get(usize::try_from(offset).ok()?..)?;
    let end = rest.iter().position(|&byte| byte == 0)?;
    Some(String::from_utf8_lossy(&rest[..end]).into_owned())
}

/// Bounds-checked reader: every overrun is `Malformed`, never a panic.
struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
    big_endian: bool,
}

macro_rules! read_number {
    ($name:ident, $type:ty) => {
        fn $name(&mut self) -> BundleResult<$type> {
            let bytes = self.array()?;
            Ok(if self.big_endian {
                <$type>::from_be_bytes(bytes)
            } else {
                <$type>::from_le_bytes(bytes)
            })
        }
    };
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8], big_endian: bool) -> Self {
        Self {
            bytes,
            position: 0,
            big_endian,
        }
    }

    fn take(&mut self, len: usize) -> BundleResult<&'a [u8]> {
        let end = self
            .position
            .checked_add(len)
            .ok_or(BundleError::Malformed)?;
        let slice = self
            .bytes
            .get(self.position..end)
            .ok_or(BundleError::Malformed)?;
        self.position = end;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> BundleResult<[u8; N]> {
        self.take(N)?.try_into().map_err(|_| BundleError::Malformed)
    }

    fn skip(&mut self, len: usize) -> BundleResult<()> {
        self.take(len).map(|_| ())
    }

    fn align(&mut self, to: usize) -> BundleResult<()> {
        let aligned = self.position.checked_next_multiple_of(to);
        self.position = aligned
            .filter(|end| *end <= self.bytes.len())
            .ok_or(BundleError::Malformed)?;
        Ok(())
    }

    read_number!(u8, u8);
    read_number!(u16, u16);
    read_number!(u32, u32);
    read_number!(i32, i32);
    read_number!(u64, u64);

    /// A count of entries that each take at least `entry_size` bytes, so a corrupt count fails fast.
    fn count(&mut self, entry_size: usize) -> BundleResult<usize> {
        let count = usize::try_from(self.i32()?).map_err(|_| BundleError::Malformed)?;
        let remaining = self.bytes.len().saturating_sub(self.position);
        if count.saturating_mul(entry_size) > remaining {
            return Err(BundleError::Malformed);
        }
        Ok(count)
    }

    fn cstr(&mut self) -> BundleResult<&'a [u8]> {
        let rest = self
            .bytes
            .get(self.position..)
            .ok_or(BundleError::Malformed)?;
        let len = rest.iter().position(|&byte| byte == 0);
        let value = self.take(len.ok_or(BundleError::Malformed)?)?;
        self.skip(1)?;
        Ok(value)
    }

    fn string(&mut self) -> BundleResult<String> {
        let len = self.count(1)?;
        Ok(String::from_utf8_lossy(self.take(len)?).into_owned())
    }
}
