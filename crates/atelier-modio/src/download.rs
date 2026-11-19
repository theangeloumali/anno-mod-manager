use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use md5::{Digest, Md5};
use reqwest::Method;
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::client::{ModioClient, PLATFORM, PLATFORM_HEADER};
use crate::error::{Error, Result};
use crate::types::{DownloadProgress, DownloadStatus, Modfile};

const VIRUS_POSITIVE_REFUSE: u8 = 1;
/// mod.io's "scan complete"; pending, too large, missing and errored scans all fail closed.
const VIRUS_SCAN_COMPLETE: u8 = 1;

impl ModioClient {
    /// Streams `modfile` into `dest_dir`, verifying its md5. A failed or
    /// mismatched download leaves nothing behind.
    pub async fn download(
        &self,
        modfile: &Modfile,
        dest_dir: &Path,
        mut on_progress: impl FnMut(DownloadProgress),
    ) -> Result<PathBuf> {
        if modfile.virus_status != VIRUS_SCAN_COMPLETE {
            return Err(Error::NotScanned {
                modfile_id: modfile.id,
                virus_status: modfile.virus_status,
            });
        }
        if modfile.virus_positive == VIRUS_POSITIVE_REFUSE {
            return Err(Error::VirusDetected {
                modfile_id: modfile.id,
            });
        }
        // file_name() keeps only the last component, so "../x.zip" cannot escape.
        let file_name = Path::new(&modfile.filename)
            .file_name()
            .ok_or(Error::InvalidFileName)?;
        tokio::fs::create_dir_all(dest_dir).await?;
        let final_path = dest_dir.join(file_name);
        let mut part_name = file_name.to_owned();
        part_name.push(".part");
        let part_path = dest_dir.join(part_name);

        let outcome = self.stream_to(modfile, &part_path, &mut on_progress).await;
        let (actual_md5, downloaded) = match outcome {
            Ok(done) => done,
            Err(error) => {
                discard(&part_path).await;
                return Err(error);
            }
        };
        if !actual_md5.eq_ignore_ascii_case(&modfile.md5) {
            discard(&part_path).await;
            return Err(Error::ChecksumMismatch {
                expected: modfile.md5.clone(),
                actual: actual_md5,
            });
        }
        tokio::fs::rename(&part_path, &final_path).await?;
        on_progress(DownloadProgress {
            status: DownloadStatus::Done,
            downloaded,
            total: downloaded,
        });
        Ok(final_path)
    }

    async fn stream_to(
        &self,
        modfile: &Modfile,
        part_path: &Path,
        on_progress: &mut impl FnMut(DownloadProgress),
    ) -> Result<(String, u64)> {
        let url = Url::parse(&modfile.binary_url)?;
        let mut request = self
            .http
            .request(Method::GET, url.clone())
            .header(PLATFORM_HEADER, PLATFORM);
        // AUTHORISED_DOWNLOADS: the bearer goes to mod.io hosts only.
        if self.is_modio_host(&url)
            && let Some(token) = self.tokens.load()?
        {
            request = request.bearer_auth(token);
        }
        let response = self.execute(request).await?;
        let total = response.content_length().unwrap_or(modfile.filesize);

        let mut file = tokio::fs::File::create(part_path).await?;
        let mut hasher = Md5::new();
        let mut downloaded = 0u64;
        let mut report = |status, downloaded| {
            on_progress(DownloadProgress {
                status,
                downloaded,
                total,
            })
        };
        report(DownloadStatus::Downloading, 0);
        let mut body = response.bytes_stream();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(Error::from_transport)?;
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            report(DownloadStatus::Downloading, downloaded);
        }
        file.flush().await?;
        // Close before rename/remove; Windows refuses either on an open handle.
        drop(file);
        report(DownloadStatus::Verifying, downloaded);
        let digest = hasher.finalize();
        let hex = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        Ok((hex, downloaded))
    }

    /// Whether the bearer may travel to `url`: the exact override origin in tests, else https mod.io only.
    fn is_modio_host(&self, url: &Url) -> bool {
        if let Some(base) = &self.base_override {
            return base.origin() == url.origin();
        }
        let Some(host) = url.host_str() else {
            return false;
        };
        url.scheme() == "https"
            && (host == "mod.io" || host.ends_with(".mod.io") || host.ends_with(".modapi.io"))
    }
}

/// Best-effort cleanup on a failure path; the original error is what the caller needs.
async fn discard(path: &Path) {
    if let Err(error) = tokio::fs::remove_file(path).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        eprintln!("atelier-modio: could not remove partial download: {error}");
    }
}

#[cfg(test)]
mod tests {
    use crate::client::{ClientConfig, ModioClient};
    use url::Url;

    fn production_client() -> ModioClient {
        ModioClient::new(ClientConfig {
            api_key: None,
            base_override: None,
        })
        .unwrap()
    }

    #[test]
    fn the_bearer_needs_https_on_a_modio_host() {
        let client = production_client();
        let https = Url::parse("https://g-11358.modapi.io/v1/files/1/download").unwrap();
        let http = Url::parse("http://g-11358.modapi.io/v1/files/1/download").unwrap();
        let lookalike = Url::parse("https://modapi.io.evil.example/x").unwrap();
        assert!(client.is_modio_host(&https));
        assert!(!client.is_modio_host(&http));
        assert!(!client.is_modio_host(&lookalike));
    }
}
