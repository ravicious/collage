'use strict';

const worker = new Worker('worker.js');
const resultImg = document.getElementById('result')

const app = Elm.Main.init({
  node: document.getElementById('app-container')
})

app.ports.sendImagesToJs.subscribe((files) => {
  generateCollage(files).catch(console.error)
})

const generateCollage = async (files) => {
  URL.revokeObjectURL(resultImg.src);
  resultImg.src = "";

  const imageArrays = await Promise.all(
    files.map((file) => file.arrayBuffer().then((buffer) => new Uint8Array(buffer)))
  )

  console.time('generate_layout');
  const resultArray = await generate_layout(imageArrays);
  console.timeEnd('generate_layout');

  resultImg.src = URL.createObjectURL(
    new Blob([resultArray.buffer], {type: 'image/jpg'})
  );

  app.ports.imageProcessorStatus.send("done");
}

// Wrapping the message passing in a promise.
// The worker code is simple enough that we can let ourselves do that.
const generate_layout = (imageArrays) => new Promise((resolve, reject) => {
  worker.onmessage = (event) => { resolve(event.data) }
  worker.postMessage(
    ['generate_layout', imageArrays],
    imageArrays.map((imageArray) => imageArray.buffer)
  )
})
const render_specific_layout = (layoutBlueprint, imageArrays) => new Promise((resolve, reject) => {
  worker.onmessage = (event) => { resolve(event.data) }
  worker.postMessage(
    ['render_specific_layout', layoutBlueprint, imageArrays],
    imageArrays.map((imageArray) => imageArray.buffer)
  )
})

const orientationTest = async () => {
  console.time('orientation test');

  const loadFile = (n) => fetch(`vendor/orientation-test/f${n}t.jpg`)
    .then((response) => response.blob())
    .then((blob) => blob.arrayBuffer())
    .then((buffer) => new Uint8Array(buffer));
  const pairs = [[1, 2], [3, 4], [5, 6], [7, 8]];

  for (const [n1, n2] of pairs) {
    const result = await generate_layout([await loadFile(n1), await loadFile(n2)]);
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

const layoutTest = async () => {
  const loadFile = (n) => fetch(`test-images/${n}`)
    .then((response) => response.blob())
    .then((blob) => blob.arrayBuffer())
    .then((buffer) => new Uint8Array(buffer));

  // digraph {
  //     0 [ label = "Horizontal" ]
  //     1 [ label = "Vertical" ]
  //     2 [ label = "Horizontal" ]
  //     3 [ label = "Vertical" ]
  //     4 [ label = "Vertical" ]
  //     5 [ label = "Horizontal" ]
  //     6 [ label = "Image(200, 140)" ]
  //     7 [ label = "Image(175, 175)" ]
  //     8 [ label = "Image(306, 220)" ]
  //     9 [ label = "Image(202, 192)" ]
  //     10 [ label = "Image(200, 302)" ]
  //     11 [ label = "Image(170, 200)" ]
  //     12 [ label = "Image(170, 170)" ]
  //     0 -> 1 [ ]
  //     1 -> 2 [ ]
  //     0 -> 3 [ ]
  //     1 -> 4 [ ]
  //     4 -> 5 [ ]
  //     2 -> 6 [ ]
  //     2 -> 7 [ ]
  //     3 -> 8 [ ]
  //     3 -> 9 [ ]
  //     4 -> 10 [ ]
  //     5 -> 11 [ ]
  //     5 -> 12 [ ]
  // }
  // Canvas dimensions: Dimensions { width: 506, height: 502 }
  const layoutBlueprint = {
    graph_representation: [
      ["H", [1, 3]],
      ["V", [2, 4]],
      ["H", []],
      ["V", []],
      ["V", [5]],
      ["H", []],
    ],
    width: 506,
    height: 502,
  }

  const images = await Promise.all([
    '140.jpg', '175.jpg', '220.jpg', '192.jpg', '302.jpg', '200.jpg', '170.jpg'
  ].map((name) => loadFile(name)))

  const resultArray = await render_specific_layout(layoutBlueprint, images);

  resultImg.src = URL.createObjectURL(
    new Blob([resultArray.buffer], {type: 'image/jpg'})
  );
}
