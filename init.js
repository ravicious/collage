'use strict';

const worker = new Worker('worker.js');
const resultImg = document.getElementById('result')

const app = Elm.Main.init({
  node: document.getElementById('app-container')
})

app.ports.sendImagesToJs.subscribe((files) => {
  generateCollage(files).catch(console.error)
})

resultImg.onload = function() {
  console.log('revoking Object URL')
  URL.revokeObjectURL(this.src);
}

const generateCollage = async (files) => {
  resultImg.src = "";

  const imageArray1 = new Uint8Array(await files[0].arrayBuffer());
  const imageArray2 = new Uint8Array(await files[1].arrayBuffer());

  console.time('process_images');
  const resultArray = await process_images(imageArray1, imageArray2);
  console.timeEnd('process_images');

  resultImg.src = URL.createObjectURL(
    new Blob([resultArray.buffer], {type: 'image/jpg'})
  );

  app.ports.imageProcessorStatus.send("done");
}

// Wrapping the message passing in a promise.
// The worker code is simple enough that we can let ourselves do that.
const process_images = (imageArray1, imageArray2) => new Promise((resolve, reject) => {
  worker.onmessage = (event) => {
    resolve(event.data)
  }
  worker.postMessage([imageArray1, imageArray2], [imageArray1.buffer, imageArray2.buffer])
})

const orientationTest = async () => {
  console.time('orientation test');

  const loadFile = (n) => fetch(`vendor/orientation-test/f${n}t.jpg`)
    .then((response) => response.blob())
    .then((blob) => blob.arrayBuffer())
    .then((buffer) => new Uint8Array(buffer));
  const pairs = [[1, 2], [3, 4], [5, 6], [7, 8]];

  for (const [n1, n2] of pairs) {
    const result = await process_images(await loadFile(n1), await loadFile(n2));
    const img = document.createElement('img');
    img.onload = function() {
      URL.revokeObjectURL(this.src);
    }
    img.src = URL.createObjectURL(
      new Blob([result.buffer], {type: 'image/jpg'})
    );

    document.body.appendChild(img);
  }

  console.timeEnd('orientation test');
}
