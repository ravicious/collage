'use strict';

const worker = new Worker('worker.js');
const resultImg = document.getElementById('result')

const app = Elm.Main.init({
  node: document.getElementById('app-container')
})

app.ports.sendImagesToJs.subscribe((files) => {
  generateCollage(files).catch(console.error)
})

const benchmarkSeed = 1338;

worker.onmessage = (event) => {
  if (event.data[0] == 'ready') {
    if (window.location.hash.includes("blueprintLayoutTest")) {
      blueprintLayoutTest()
    }

    if (window.location.hash.includes("randomLayoutTest")) {
      randomLayoutTest()
    }

    if (window.location.hash.includes("benchmark")) {
      prepareBenchmark().then(() => {
        console.log("Benchmark prepared")
      })
    }
  }
}

const generateCollage = async (files, seed) => {
  URL.revokeObjectURL(resultImg.src);
  resultImg.src = "";

  const imageArrays = await Promise.all(
    files.map((file) => file.arrayBuffer().then((buffer) => new Uint8Array(buffer)))
  )

  console.time('generate_layout');
  const resultArray = await generate_layout(imageArrays, seed);
  console.timeEnd('generate_layout');

  resultImg.src = URL.createObjectURL(
    new Blob([resultArray.buffer], {type: 'image/jpg'})
  );

  app.ports.imageProcessorStatus.send("done");
}

// Wrapping the message passing in a promise.
// The worker code is simple enough that we can let ourselves do that.
const generate_layout = (imageArrays, seed) => new Promise((resolve, reject) => {
  worker.onmessage = (event) => { resolve(event.data) }
  worker.postMessage(
    ['generate_layout', imageArrays, seed],
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

const blueprintLayoutTest = async () => {
  const loadFile = (n) => fetch(`test-images/${n}`)
    .then((response) => response.blob())
    .then((blob) => blob.arrayBuffer())
    .then((buffer) => new Uint8Array(buffer));

  // digraph {
  //     0 [ label = "Horizontal" ]
  //     1 [ label = "Vertical" ]
  //     2 [ label = "Horizontal" ]
  //     3 [ label = "Horizontal" ]
  //     4 [ label = "Horizontal" ]
  //     5 [ label = "Vertical" ]
  //     6 [ label = "Image(175, 175)" ]
  //     7 [ label = "Image(200, 302)" ]
  //     8 [ label = "Image(202, 192)" ]
  //     9 [ label = "Image(170, 200)" ]
  //     10 [ label = "Image(200, 140)" ]
  //     11 [ label = "Image(306, 220)" ]
  //     12 [ label = "Image(170, 170)" ]
  //     0 -> 1 [ ]
  //     1 -> 2 [ ]
  //     1 -> 3 [ ]
  //     2 -> 4 [ ]
  //     0 -> 5 [ ]
  //     2 -> 6 [ ]
  //     3 -> 7 [ ]
  //     3 -> 8 [ ]
  //     4 -> 9 [ ]
  //     4 -> 10 [ ]
  //     5 -> 11 [ ]
  //     5 -> 12 [ ]
  // }
  // Canvas dimensions: Dimensions { width: 370, height: 642 }
  const layoutBlueprint = {
    graph_representation: [
      ["H", [1, 5]],
      ["V", [2, 3]],
      ["H", [4]],
      ["H", []],
      ["H", []],
      ["V", []],
    ],
    width: 370,
    height: 642,
  }

  const images = await Promise.all([
    '175.jpg', '170.jpg', '220.jpg', '192.jpg', '200.jpg', '140.jpg', '302.jpg'
  ].map((name) => loadFile(name)))

  const resultArray = await render_specific_layout(layoutBlueprint, images);

  resultImg.src = URL.createObjectURL(
    new Blob([resultArray.buffer], {type: 'image/jpg'})
  );
}

const randomLayoutTest = async () => {
  const loadFile = (n) => fetch(`test-images/${n}`)
    .then((response) => response.blob());

  const files = await Promise.all([
    '140.jpg', '175.jpg', '220.jpg', '192.jpg', '302.jpg', '200.jpg', '170.jpg'
  ].map((name) => loadFile(name)))

  generateCollage(files).catch(console.error)
}

let benchmarkFiles = null;

const prepareBenchmark = async () => {
  const scriptPromise = loadScript('vendor/lodash.min.js')
    .then(() => loadScript('vendor/platform.js'))
    .then(() => loadScript('vendor/benchmark.js'))

  const loadFile = (n) => fetch(`test-images/${n}`)
    .then((response) => response.blob());

  benchmarkFiles = await Promise.all([
    '140.jpg', '175.jpg', '220.jpg', '192.jpg', '302.jpg', '200.jpg', '170.jpg'
  ].map((name) => loadFile(name)))

  await scriptPromise
}

const benchmark = async (seed = benchmarkSeed) => {
  const suite = new Benchmark.Suite;
  suite
    .add('Generate layout', {
      defer: true,
      fn: (deferred) => {
        generateCollage(benchmarkFiles, seed).then(() => {
          deferred.resolve();
        },
          console.error)
      }
    })
    .on('cycle', function(event) {
      console.log(String(event.target));
      console.log(event.target.stats.mean)
    })
    .on('complete', function() {
      console.log('Benchmark finished')
    })
    .run();
}

const benchmarkOnce = async (seed = benchmarkSeed) => {
  generateCollage(benchmarkFiles, seed).catch(console.error);
}

const loadScript = (path) => new Promise((resolve, reject) => {
  let script = document.createElement('script');
  script.onload = () => { resolve() };
  script.onerror = () => { reject() };
  script.setAttribute('src', path);
  document.body.appendChild(script);
})
