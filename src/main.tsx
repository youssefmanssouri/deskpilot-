import React from "react";
import ReactDOM from "react-dom/client";
// Fonts are bundled into the app by Vite (no network requests at runtime).
import "@fontsource-variable/inter";
import "@fontsource-variable/manrope";
import "./App.css";
import App from "./App";
import { LauncherApp } from "./launcher/LauncherApp";

// The same bundle serves two windows: the main window and the quick launcher
// (declared in tauri.conf.json with url "index.html?window=launcher").
const isLauncherWindow = new URLSearchParams(window.location.search).get("window") === "launcher";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{isLauncherWindow ? <LauncherApp /> : <App />}</React.StrictMode>,
);
