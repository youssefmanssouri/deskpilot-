import React from "react";
import ReactDOM from "react-dom/client";
// Fonts are bundled into the app by Vite (no network requests at runtime).
import "@fontsource-variable/inter";
import "@fontsource-variable/manrope";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
