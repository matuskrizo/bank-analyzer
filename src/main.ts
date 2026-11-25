import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

// Svelte 5: komponent sa pripája cez `mount` (nie `new App(...)` ako v Svelte 4).
const app = mount(App, { target: document.getElementById("app")! });

export default app;
