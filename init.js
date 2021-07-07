'use strict';

const app = Elm.Main.init({
  node: document.getElementById('app-container')
})

app.ports.sendImagesToJs.subscribe((files) => {
  console.log("Received files from elm", files)
  generateCollage(files).catch(console.error)
})

const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');

const generateCollage = async (files) => {
  const image1 = await loadImage(files[0]);
  const image2 = await loadImage(files[1]);

  drawImages(image1, image2);
}

const loadImage = (file) => new Promise((resolve, reject) => {
  const reader = new FileReader()

  reader.addEventListener("load", () => {
    const image = new Image()
    image.onload = () => resolve(image)
    image.onerror = (error) => reject(error)
    image.src = reader.result
  }, false)

  reader.addEventListener("error", () => {
    reject(reader.error)
  })

  reader.readAsDataURL(file)
})

function drawImages(image1, image2) {
  console.log('Calculating sizes');

  const fw = image1.naturalWidth;
  const fh = image1.naturalHeight;
  const sw = image2.naturalWidth;
  const sh = image2.naturalWidth;

  // Create a solver
  var solver = new kiwi.Solver();

  // Create edit variables
  const fx = new kiwi.Variable();
  const fy = new kiwi.Variable();
  const sx = new kiwi.Variable();
  const sy = new kiwi.Variable();
  const variables = [fx, fy, sx, sy];

  for (const variable of variables) {
    solver.addEditVariable(variable, kiwi.Strength.strong);
    // Bigger than zero.
    solver.addConstraint(new kiwi.Constraint(
      new kiwi.Expression(variable), kiwi.Operator.Ge, 0,
      kiwi.Strength.required
    ))
  }

  // Top right corner of the first image should touch the top left corner of the second image.
  solver.addConstraint(new kiwi.Constraint(
    new kiwi.Expression(fx, fw, [-1, sx]),
    kiwi.Operator.Eq
  ))
  solver.addConstraint(new kiwi.Constraint(
    new kiwi.Expression([-1, fy], sy),
    kiwi.Operator.Eq
  ))

  // Solve the constraints
  solver.updateVariables();
  const namedVariables = {fx, fy, sx, sy};
  for (const key in namedVariables) {
    console.log(`${key} = ${namedVariables[key].value()}`)
  }

  console.log("Drawing image");
  // We assume top right corner of the first image touches the top left corner of the second image.
  canvas.width = image1.naturalWidth + image2.naturalWidth;
  canvas.height = Math.max(image1.naturalHeight, image2.naturalHeight);

  ctx.clearRect(0, 0, canvas.width, canvas.height);

  // Draw background.
  ctx.fillStyle = "white";
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  ctx.drawImage(image1, fx.value(), fy.value());
  ctx.drawImage(image2, sx.value(), sy.value());
}

document.getElementById('download').addEventListener('click', function (event) {
  this.href = canvas.toDataURL('image/jpeg');
}, false);
