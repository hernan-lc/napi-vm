import { render } from "preact";
import { App } from "./components/App.tsx";
import "./style.css";

const root = document.getElementById("app");
if (root) render(<App />, root);
