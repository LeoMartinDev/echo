import "./app.css";
import { mount } from "svelte";
import { installDevMockIfNeeded } from "./lib/devMock";
import App from "./App.svelte";

installDevMockIfNeeded();

export default mount(App, { target: document.getElementById("app")! });
