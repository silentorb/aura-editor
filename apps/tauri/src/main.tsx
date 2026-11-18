import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import {
  localStorageColorSchemeManager,
  MantineProvider,
} from "@mantine/core";
import "@mantine/core/styles.css";
import App from "./App";
import { theme } from "./theme";

const colorSchemeManager = localStorageColorSchemeManager({
  key: "aura-editor-color-scheme",
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <MantineProvider
      theme={theme}
      defaultColorScheme="dark"
      colorSchemeManager={colorSchemeManager}
    >
      <App />
    </MantineProvider>
  </StrictMode>,
);
