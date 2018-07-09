# Car Trips & Pathfinding

## Philosophy

## Decisions

## Implementation Philosophy

* Keep pathfinding information local and dynamically changeable
* Make use of the "highway effect" (for most of each trip, only the rough source and rough destination determine the typically important main connections to choose)
* Be able to create a trip from household to household (not necessarily precise addresses)

## Implementation Decisions

* Trips are a persistent representation of each ongoing trip (since cars themselves are not actors, but exist on road actors, or are exchanged as messages)
   * Trips can be created from and to "rough" locations (such as households), which are then first resolved to precise locations on the road network
* Use a simple two-layer landmark model for the pathfinding itself:
   * "nodes" (individual lanes) and "landmarks" (groups of lanes, represented as one rough destination)
   * nodes store closest-next-hop information to *all* landmarks, but only to nodes up to a certain distance
   * if a node is too far away, first navigate towards its parent landmark, until a direct next-hop is known
   * lanes organically join/leave landmarks, keeping landmarks continuous each, and of roughly equal group size
   * lanes exchange information about changed/updated next-hops with their local neighbors (similar to router table updates), forming an eventually converging network
