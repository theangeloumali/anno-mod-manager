import "@fontsource-variable/archivo/wdth.css";
import "@fontsource-variable/jetbrains-mono";
import "./styles/app.css";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app.tsx";

const rootElement = document.getElementById("root");
if (!rootElement) throw new Error("index.html is missing #root");

const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false } },
});

createRoot(rootElement).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </StrictMode>,
);
