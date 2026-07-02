import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { AppProviders } from "./providers";
import { CommandCenterApp } from "./CommandCenterApp";
import "../styles.css";

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("TokenStack root element is missing.");
}

createRoot(rootElement).render(
  <StrictMode>
    <AppProviders>
      <CommandCenterApp />
    </AppProviders>
  </StrictMode>,
);
