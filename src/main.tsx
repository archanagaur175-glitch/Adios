import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { useSettingsStore } from "@/store/settingsStore";

void useSettingsStore.getState().hydrate();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
