# descartes

descartes is a...

- [X] Error-tolerant 2D geometry engine
- [X] that allows for arbitrary error tolerances
- [X] dealing with both floating-point inaccuracies and much larger user input inaccuracies

With the following primitives:

- [X] line
- [X] circle
- [X] line/circle segment

And the following compound objects:

- [X] Path (continuous concatenation of line/circle segments)
- [ ] Shape (Path outline with 0..n Path holes)
- [X] Band (a path with a thickness)

It offers...

- [ ] Reexported 2D & 3D Point/Vector operations from `nalgebra`
- [X] Projections from and onto lines, circles, segments, paths, bands
- [X] Intersections
   - [X] between lines & circles
   - [X] between line/circle segments
   - [X] between paths
- [X] Axis-aligned bounding boxes for
   - [X] line/circle segments
   - [X] paths
- [ ] Boolean operations between shapes
- [X] Orthogonal offsets of segments
- [ ] True Orthogonal offsets of paths (without producing self-intersections)
- [X] A `RoughEq` Trait for comparing things within a tolerance, with implementations for `P2, P3` and `V2, P3`

It internally uses...

- [X] "Thick" primitives for tolerances

descartes is named after [René Descartes](https://en.wikipedia.org/wiki/René_Descartes), the father of analytical geometry.