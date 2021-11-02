# collage

## Rationale

I often post photos of my cat on Discord, but I don't like how Discord displays multiple uploaded
photos, particularly in portrait orientation.

collage lets you upload two photos and then it stitches them together. The result will always be a
landscape image, unless the input is two landscape photos then collage will make a portrait image.
The bigger image will be scaled down to match the smaller in width or height.

When uploading more than two photos, collage uses an algorithm described in [Photo Layout with a
Fast Evaluation Method and Genetic Algorithm](https://www.researchgate.net/publication/233529670_Photo_Layout_with_a_Fast_Evaluation_Method_and_Genetic_Algorithm).

It's not yet fully implemented and has some quirks, particularly around actually rendering the
photos, but it tries to find the optimal layout. The optimal layout is one in which the images keep
their original dimensions.

## Development

To run this thing in development mode:

```
cargo install wasm-pack
yarn
yarn dev-server
```
