'use strict';

import {default as initWasm, setup, process_images} from './image-processor/pkg/image_processor.js';

(async () => {
  await initWasm();
  setup();
})();

const app = Elm.Main.init({
  node: document.getElementById('app-container')
})

app.ports.sendImagesToJs.subscribe((files) => {
  generateCollage(files).catch(console.error)
})

const generateCollage = async (files) => {
  document.getElementById('result').src = "";

  const imageArray1 = new Uint8Array(await files[0].arrayBuffer());
  const imageArray2 = new Uint8Array(await files[1].arrayBuffer());

  console.time('process_images');
  const resultArray = process_images(imageArray1, imageArray2);
  console.timeEnd('process_images');

  document.getElementById('result').src = URL.createObjectURL(
    new Blob([resultArray.buffer], {type: 'image/jpg'})
  );

  app.ports.imageProcessorStatus.send("done");
}

const orientationTest = async () => {
  const loadFile = (n) => fetch(`vendor/orientation-test/f${n}t.jpg`)
    .then((response) => response.blob())
    .then((blob) => blob.arrayBuffer())
    .then((buffer) => new Uint8Array(buffer));
  const pairs = [[1, 2], [3, 4], [5, 6], [7, 8]];

  for (const [n1, n2] of pairs) {
    const result = process_images(await loadFile(n1), await loadFile(n2));
    const img = document.createElement('img');
    img.src = URL.createObjectURL(
      new Blob([result.buffer], {type: 'image/jpg'})
    );

    document.body.appendChild(img);
  }
}
window.orientatationTest = orientationTest;
