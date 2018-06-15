# Foliage Rendering

## Prior Art & Inspiration

* [/r/proceduralgeneration](https://www.reddit.com/r/proceduralgeneration/search?q=trees&restrict_sr=on) - Search for "trees"
* [IXORA](http://www.ixora.org/) (Private company)
  * Tensor-based graph computations
  * [CG growing basil](https://www.youtube.com/watch?v=qdd3M6ilc2M&t=0s&index=2&list=PLT48XojLMCzNCb73z19LFQlSNEMIQdGo-) (YouTube)

### Techniques

* [Lies and Imposters](https://paroj.github.io/gltut/Illumination/Tutorial%2013.html) Chapter 13 - OpenGL Billboarding/Imposter tutorial (Learning Modern 3D Graphics Programming)
* [Billboarding](http://www.flipcode.com/archives/Billboarding-Excerpt_From_iReal-Time_Renderingi_2E.shtml) excerpt (Real-Time Rendering 2E)
* [Impostors, Pseudo-instancing and Image Maps for GPU Crowd Rendering](http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.98.3592&rep=rep1&type=pdf) (The International Journal of Virtual Reality)
* [Dynamic 2D Imposters](https://www.gamasutra.com/view/feature/130911/dynamic_2d_imposters_a_simple_.php) generation (Gamasutra)
* [Imposters Made Easy](https://software.intel.com/en-us/articles/impostors-made-easy) for procedurally generated trees (Intel)
  * > ... as few as four different trees, which are then rotated and scaled differently, can result in an impressively realistic forest.
  * We could render a small handful of generated plants per species for "good enough" variation

## Philosophy

* A plant is always growing, unless:
  * It is currently [Dormant](../lifecycle#dormancy), or
  * It is [Dead](../lifecycle#death)
* Each stage of [Growth](../lifecycle#growth) is distinct
* Growing plants have different heights, shapes, and colors.

A plant in good health is visually distinct from an unhealthy or dying plant

* Trunks and branches are solid brown (duh 😝)
* Leaves and stems are green (also duh 😝)
* Flowers vary in color
  * Usually the same color for a single plant

Use varying shades of brown and green in the same and for different plants to give the city some visual panache

### Dying and Dead Plants

#### Dying ([Unhealthy](../health))

* Dying trees, shrubs, and bushes will begin to lose their leaves and change color
* Dying conifers' needles turn brown then begin to fall off

#### Dead

* Dead trees, shrubs, and bushes lose all their leaves
* Dead conifers' needles are brown and have fallen off

### Questions

* How is a dead plant's color distinct from a healthy or dying plant?
* Do we render fallen dead leaves?  

## Decisions

## Implementation Philosophy

## Implementation Decisions
