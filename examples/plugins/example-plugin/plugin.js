import { readText, writeText } from "napi:fs";
import { join } from "napi:path";

export default class ExamplePlugin {
  onLoad(context) {
    this.config = JSON.parse(readText("./config.json"));
    this.banner = readText("./assets/banner.txt");

    writeText(
      join("./cache", "status.json"),
      JSON.stringify({
        loaded: true,
        plugin: context.name,
        version: context.version,
        greeting: this.config.greeting,
      })
    );

    return this.config.greeting;
  }

  onUnload(context) {
    return { config: this.config, reason: context.reason };
  }

  onReload(context, previousState) {
    if (previousState && previousState.config) {
      this.config = previousState.config;
    } else {
      this.config = JSON.parse(readText("./config.json"));
    }
    writeText(
      join("./cache", "status.json"),
      JSON.stringify({ reloaded: true, plugin: context.name })
    );
    return this.config.greeting;
  }
}
