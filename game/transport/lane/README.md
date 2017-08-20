# Road Lanes

## Design Philosophy

* The complex fabric of real road geometry goes beyond "streets" and "intersections", which at best can be understood as loose groups of related lanes `(emergence)` `(actual problem)`

## Design Decision

* The atomic unit of road geometry in Citybound is the individual lane.

## Implementation Philosophy

* Really make individual lanes the main abstraction, only group them into higher abstractions when absolutely necessary.
* If possible, only handle a group of "nearby/related" lanes temporarily while using a tool
* Also see ~[Road Planning](../planning/README.md)~

## Implementation Decisions

* Each lane keeps track of
   * it's connectivity to preceeding/following lanes (connected to it's start or end)
   * lanes it overlaps, distinguishing between nearly-parallel overlaps and opposing overlaps
* Special case: two parallel lines that can be merged between are not directly connected
   * instead, an invisible "transfer lane" is placed along their range of overlap
   * the lanes on either side have a special parallel overlap relation ship to the transfer lane only, not to each other.
   * the reason for this is that merging car behaviour is a more complex special case
* Also see ~[Car Microtraffic](../microtraffic/README.md)~
