import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import "monaco-editor/min/vs/editor/editor.main.css";
import "./assets/styles.css";

const app = createApp(App);
app.use(createPinia());
app.mount("#app");
