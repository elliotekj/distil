# Distil: a colour palette generator [![AppVeyor Build Status](https://ci.appveyor.com/api/projects/status/github/elliotekj/distil?branch=master&svg=true)](https://ci.appveyor.com/project/elliotekj/distil)

> WIP - Not ready for production use

Distil is a Rust library that creates a colour palette from the most frequently
used colours in an image.

| [**Examples**](https://github.com/elliotekj/distil#examples) | [**How does it work?**](https://github.com/elliotekj/distil#how-does-it-work) | [**1.0 checklist**](https://github.com/elliotekj/distil#10-checklist) |

## How does it work?

Lets go through it step by step.

##### Downsampling

Distil starts by scaling the image down—whilst preserving its aspect ratio—until
it consists of no more that 1000 pixels.

##### Quantization

From there, it's run through the [NeuQuant
algorithm](https://scientificgems.wordpress.com/stuff/neuquant-fast-high-quality-image-quantization/)
to reduce it to an 8-bit image composed of 256 colours.

##### Colour differentiation

Next, the number of appearances each unique colour makes in the image is counted
and put into a `Vec`. That `Vec` is then sorted by most frequently used colour
to least frequently used. Lets name it `palette` for the sake of clarity going
forward.

A separate `Vec`, which we'll dub `refined_palette`, is now created and it's
from that `Vec` that the final palette will be built.

Starting from the top (i.e. the most frequently used colour), the program then
works its way down `palette` comparing how similar `x` — the current colour from
`palette` being processed — is by human eye standards to each and every colour
in `refined_palette`.

If there are no colours similar to `x` already in `refined_palette`, then `x`
gets added to `refined_palette`. If, however, there is a similar colour already
in `refined_palette` then an average is made of the two colours which takes into account
their frequency in the image.

The difference between two colours from the human eye perspective is calculated
with the [CIEDE2000 colour difference
algorithm](https://en.wikipedia.org/wiki/Color_difference#CIEDE2000).

##### Colour weight

With all of the colours now processed, the colours in the `refined_palette`
`Vec` are once again sorted from most frequently used to least frequently used.
An important note here though is that the sorting is done taking into account
the occurrence count of each of the pixels that were deemed similar in colour
and merged together when building `refined_palette`.

## 1.0 checklist

- [x] Handle a pure-white or pure-black image being processed. Pixels that are
  too dark or too light to be interesting in a palette currently get filtered
  out during quantization.
- [ ] Add a way to create a distillation from multiple `Distil`s. i.e. A way to
  get one `Distil` from the colours of multiple images.

## Examples

![](https://github.com/elliotekj/distil/blob/master/images/img-1.jpg?raw=true)
![](https://github.com/elliotekj/distil/blob/master/images/img-1-palette.png?raw=true)

<br>

![](https://github.com/elliotekj/distil/blob/master/images/img-3.jpg?raw=true)
![](https://github.com/elliotekj/distil/blob/master/images/img-3-palette.png?raw=true)

<br>

![](https://github.com/elliotekj/distil/blob/master/images/img-4.jpg?raw=true)
![](https://github.com/elliotekj/distil/blob/master/images/img-4-palette.png?raw=true)

<br>

![](https://github.com/elliotekj/distil/blob/master/images/img-6.jpg?raw=true)
![](https://github.com/elliotekj/distil/blob/master/images/img-6-palette.png?raw=true)

<br>

![](https://github.com/elliotekj/distil/blob/master/images/img-5.jpg?raw=true)
![](https://github.com/elliotekj/distil/blob/master/images/img-5-palette.png?raw=true)
