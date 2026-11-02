import React from "react";
import ReactDOM from "react-dom/client";

import { App } from "./App";
import { loadSettings, logErrors } from "./settings";
import "./styles/index.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("index.html is missing its root element");
}

logErrors();

void loadSettings().then(() => {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
