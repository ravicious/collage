'use strict';

import {default as initWasm, setup, process_images} from './image-processor/pkg/image_processor.js';

initWasm().then(() => setup());

const app = Elm.Main.init({
  node: document.getElementById('app-container')
})

app.ports.sendImagesToJs.subscribe((files) => {
  generateCollage(files).catch(console.error)
})

const generateCollage = async (files) => {
  const imageArray1 = new Uint8Array(await files[0].arrayBuffer());
  const imageArray2 = new Uint8Array(await files[1].arrayBuffer());

  const resultArray = process_images(imageArray1, imageArray2);

  document.getElementById('result').src = URL.createObjectURL(
    new Blob([resultArray.buffer], {type: 'image/jpg'})
  );

  app.ports.imageProcessorStatus.send("done");
}
