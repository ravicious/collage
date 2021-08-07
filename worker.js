'use strict';

importScripts('image-processor/pkg/image_processor.js');

const initWasm = wasm_bindgen;
const {process_images} = wasm_bindgen;

initWasm('image-processor/pkg/image_processor_bg.wasm');

onmessage = function(event) {
  const result = process_images(event.data[0], event.data[1]);
  postMessage(result, [result.buffer]);
}
