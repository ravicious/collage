'use strict';

importScripts('image-processor/pkg/image_processor.js');

const initWasm = wasm_bindgen;
const {setup, process_images} = wasm_bindgen;

(async () => {
  await initWasm('image-processor/pkg/image_processor_bg.wasm');
  setup();
})();

onmessage = function(event) {
  const result = process_images(event.data[0], event.data[1]);
  postMessage(result, [result.buffer]);
}
