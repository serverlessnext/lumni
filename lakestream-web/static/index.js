import init, { main } from "./pkg/lakestream_web.js";

async function main_js() {
  await init();
  main();
}

main_js();
