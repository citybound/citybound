# Car Pathfinding

## Design Philosophy

## Design Decisions

## Implementation Philosophy

* Keep pathfinding information local and dynamically changeable
* Make use of the "highway effect" (for most of each trip, only the rough source and rough destination determine the typically important main connections to choose)

## Implementation Decisions

* Use a simple two-layer landmark model:
   * "nodes" (individual lanes) and "landmarks" (groups of lanes, represented as one rough destination)
   * nodes store closest-next-hop information to *all* landmarks, but only to nodes up to a certain distance
   * if a node is too far away, first navigate towards its parent landmark, until a direct next-hop is known
   * lanes organically join/leave landmarks, keeping landmarks continouous each, and of roughly equal group size
   * lanes exchange information about changed/updated next-hops with their local neighbors (similar to router table updates), forming an eventually converging network
