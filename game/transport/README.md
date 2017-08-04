# Transport

## Design Philosophy

* Traffic is the pulse of a city `(city = organism)`
* Traffic is a chaotic emergent phenomenon `(emergence)`
* Traffic shapes the city and the city shapes traffic `(interdependent systems)`
* Traffic shapes lives and lives shape traffic `(interdependent systems)`
* People and goods use a multitude/combinations of transport systems `(diverse individuals)`
* Simulation goal: trips are successful often enough and efficient enough to power all economic transaction realistically
* Aesthetic goal: movement patterns of pedestrians & vehicles evoke "exactly the right feeling" `(diverse individuals)` `(city = anthill)` `(vastness)` `(flows)`
    * individuals are the most visible when they are on the move
    * crowds should look antsy, sprawling, colorful 
    * cars can flow nicely but also get really pushy, different types/sizes of cars should have obviously different silhouettes and movement patterns
    * trains and boats are heavy and careful, they have impressive momentum
    * planes are graceful and precise but need a lot of space

## Design Decisions

* Always model each trip individually and microscopically (never use statistical models) `(actual problem)` `(emergence)`
    * Physical motion through time/space
    * Realistic use of potentially several modes of transport in sequence
    * Counteract time-compression with generous behaviour parameters (but never cheat, i.e. teleport) `(actual problem)`
* Pedestrians, road traffic, public transport and freight transport of all kinds are all equally important
* Parking is a core problem & component of road traffic

## Implementation Philosophy

* All kinds of traffic are "vessels/bodies moving along the edges of a transport network graph"

## Implementation Decisions

* The lane is the atomic unit, roads and intersections are just collections of lanes `(emergence)` `(flexibility)`
* Optimal granularity of traffic simulation: simulate the most strongly locally interacting traffic entities (e.g. cars on one lane) all at once within one actor (one lane = one actor)

## Parts

* Pedestrian Traffic (0% alpha)
* Road Traffic (50% alpha)
   * ~~[Road Lanes](lane/README.md)~~ & ~~[Car Microtraffic](microtraffic/README.md)~~
   * ~~[Car Pathfinding](pathfinding/README.md)~~
   * ~~[Car Trips](trips/README.md)~~
   * ~~[Rendering](rendering/README.md)~~
* Rail Traffic (0% alpha)
* Water Traffic (0% beta)
* Air Traffic (0% beta)

## Gameplay Aspects

* Planning
   * ~~[Road planning](planning/README.md)~~ (40% alpha)
* Construction
   * ~~[Road planning](construction/README.md)~~ (50% alpha)
