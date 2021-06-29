'use strict';

const app = Elm.Main.init({
  node: document.getElementById('app-container')
})

const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');


const image = new Image();
image.onload = drawImage; // Draw when image has loaded
image.src = 'test.jpg';

// Load an image of intrinsic size 300x227 in CSS pixels
// image.src = 'https://mdn.mozillademos.org/files/5397/rhino.jpg';

function drawImage() {
  console.log('Calculating sizes');

  const fw = image.naturalWidth;
  const fh = image.naturalHeight;
  const sw = fw; // Hardcoding two identical images for now.
  const sh = fh;

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

  // solver.addConstraint(new kiwi.Constraint(new kiwi.Expression([-1, right], left, width), kiwi.Operator.Eq));
  solver.addConstraint(new kiwi.Constraint(
    new kiwi.Expression(fx, fw, [-1, sx]),
    kiwi.Operator.Eq
  ))
  solver.addConstraint(new kiwi.Constraint(
    new kiwi.Expression([-1, fy], sy, sh),
    kiwi.Operator.Eq
  ))

  // Solve the constraints
  solver.updateVariables();
  const namedVariables = {fx, fy, sx, sy};
  for (const key in namedVariables) {
    console.log(`${key} = ${namedVariables[key].value()}`)
  }



  console.log("Drawing image");
  // Use the intrinsic size of image in CSS pixels for the canvas element
  canvas.width = image.naturalWidth * 3;
  canvas.height = image.naturalHeight * 3;

  ctx.fillStyle = "green";
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  // Will draw the image as 300x227, ignoring the custom size of 60x45
  // given in the constructor
  ctx.drawImage(image, fx.value(), fy.value());
  ctx.drawImage(image, sx.value(), sy.value());
  // ctx.drawImage(image, image.naturalWidth, 0);

  // To use the custom size we'll have to specify the scale parameters
  // using the element's width and height properties - lets draw one
  // on top in the corner:
  // ctx.drawImage(image, 0, 0, image.width, image.height);
}

document.getElementById('download').addEventListener('click', function (event) {
  this.href = canvas.toDataURL('image/jpeg', 0.9);
}, false);
