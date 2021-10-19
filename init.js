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

  const imageArrays = await Promise.all(
    files.map((file) => file.arrayBuffer().then((buffer) => new Uint8Array(buffer)))
  )

  console.time('process_images');
  const resultArray = await process_images(imageArrays);
  console.timeEnd('process_images');

  resultImg.src = URL.createObjectURL(
    new Blob([resultArray.buffer], {type: 'image/jpg'})
  );

  app.ports.imageProcessorStatus.send("done");
}

// Wrapping the message passing in a promise.
// The worker code is simple enough that we can let ourselves do that.
const process_images = (imageArrays) => new Promise((resolve, reject) => {
  worker.onmessage = (event) => {
    resolve(event.data)
  }
  worker.postMessage(imageArrays, imageArrays.map((imageArray) => imageArray.buffer))
})

const orientationTest = async () => {
  console.time('orientation test');

  const loadFile = (n) => fetch(`vendor/orientation-test/f${n}t.jpg`)
    .then((response) => response.blob())
    .then((blob) => blob.arrayBuffer())
    .then((buffer) => new Uint8Array(buffer));
  const pairs = [[1, 2], [3, 4], [5, 6], [7, 8]];

  for (const [n1, n2] of pairs) {
    const result = await process_images([await loadFile(n1), await loadFile(n2)]);
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
