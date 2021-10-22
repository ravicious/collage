'use strict';

importScripts('image-processor/pkg/image_processor.js');

const initWasm = wasm_bindgen;
const {generate_layout} = wasm_bindgen;

initWasm('image-processor/pkg/image_processor_bg.wasm');

onmessage = function(event) {
  const [action, ...payload] = event.data;
  console.log('Worker received a message', {action, payload});

  switch (action) {
    case 'generate_layout':
      const imageArrays = payload[0];
      const result = generate_layout(imageArrays);
      postMessage(result, [result.buffer]);
      break;
    default:
      throw new Error(`Unknown action: ${action}`)
  }
}
