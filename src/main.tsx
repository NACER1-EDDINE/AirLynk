import React from "react";
import ReactDOM from "react-dom/client";
import { MotionConfig } from "framer-motion";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <MotionConfig reducedMotion={typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "user" : "never"}>
      <App />
    </MotionConfig>
  </React.StrictMode>,
);