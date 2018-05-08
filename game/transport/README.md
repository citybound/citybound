# Transport

## Philosophy

* Traffic is the pulse of a city `(city = organism)`
* Traffic is a chaotic emergent phenomenon `(emergence)`
* Traffic shapes the city and the city shapes traffic `(interdependent systems)`
* Traffic shapes lives and lives shape traffic `(interdependent systems)`
* People and goods use a multitude/combinations of transport systems `(diverse individuals)`
* Pedestrians & vehicles move "like different kinds of swarms/liquids" `(diverse individuals)` `(city = anthill)` `(vastness)` `(flows)`
    * individuals are the most visible when they are on the move
    * crowds look antsy, sprawling, colorful 
    * cars can flow nicely but also get really pushy, different types/sizes of cars have obviously different silhouettes and movement patterns
    * trains and boats are heavy and careful, they have impressive momentum
    * planes are graceful and precise but need a lot of space

## Decisions

* Always model each trip individually and microscopically (never use statistical models) `(actual problem)` `(emergence)`
    * Physical motion through time/space
    * Realistic use of potentially several modes of transport in sequence
    * Counteract time-compression with generous behaviour parameters (but never cheat, i.e. teleport) `(actual problem) (time compression)`
* Pedestrians, road traffic, public transport and freight transport of all kinds are all equally important
* Parking is a core problem & component of road traffic

## Implementation Philosophy

* Simulation goal: trips are successful often enough and efficient enough to power all economic transaction realistically
* All kinds of traffic are "vessels/bodies moving along the edges of a transport network graph"

## Implementation Decisions

* Optimal granularity of traffic simulation: simulate the most strongly locally interacting traffic entities (e.g. cars on one lane) all at once within one actor (one lane = one actor)

## Parts

* Planning
   * [Transport Planning](transport_planning/README.md) (40% alpha)
   * ~~[Rail & Tram Planning]()~~ (0% alpha)
   * ~~[Channel/Waterway Planning]()~~ (0% alpha)
   * ~~[Airport Planning]()~~ (0% alpha)
   * ~~[Public Transport Planning]()~~ (0% alpha)
* Construction
   * [Road Construction](construction/README.md) (50% alpha)
* Road Trips & Pathfinding
   * [Road Trips & Pathfinding](pathfinding/README.md) (10% alpha)
   * ~~[Shared & Scheduled Vehicles]()~~ (0% alpha)
* Traffic
   * Pedestrian Traffic (0% alpha)
      * ~~[Built & Organic Paths]()~~, ~~[Pedestrian Path Microtraffic]()~~
   * Road Traffic (50% alpha)
      * [Road Lanes](lane/README.md) & [Car Microtraffic](microtraffic/README.md)
      * ~~[Rendering](rendering/README.md)~~
      * ~~[Parking]()~~ (0% alpha)
   * Rail Traffic (0% alpha)
      * ~~[Train Microtraffic]()~~
      * ~~[Stations]()~~
   * Water Traffic (0% beta)
      * ~~[Vessel Microtraffic]()~~
      * ~~[Ports]()~~
   * Air Traffic (0% beta)
      * ~~[Airplane & Taxiing Microtraffic]()~~
      * ~~[Airports]()~~

## Gameplay & Skills

* "Building a city"
   * "Planning"
      * "Transport Infrastructure Planning"
         * ["Road Planning"](planning/README.md#road-planning-skill)
         * ~~["Rail & Tram Planning"]()~~
         * ~~["Channel/Waterway Planning"]()~~
         * ~~["Airport Planning"]()~~
         * ~~["Public Transport Planning"]()~~
