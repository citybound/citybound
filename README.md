<img src="cb.png" alt="Citybound" width="192"/>

Citybound is a city building game with a focus on realism, collaborative planning and simulation of microscopic details. It is independently developed, open source and funded through Patreon.

* [Homepage](http://cityboundsim.com)
* [Latest live builds](http://aeplay.co/citybound-livebuilds)
* [LICENSE](LICENSE.txt) (AGPL)
* [Contributing & Development](CONTRIBUTING.md)

# Citybound

**This is a living and open document, find out [how to contribute your suggestions](CONTRIBUTING.md).**

Citybound is a city building game with a focus on realism, collaborative planning and simulation of microscopic details.

## Subtopics

* Simulated World
  * [Simulation Core](#simulation-core)
  * [Transport](#transport)
  * [Economy & Household Behaviour](#economy--household-behaviour)
  * ~~[Construction](construction/README.md)~~
  * [Environment](#environment)
* Player Interaction
  * [Planning](#planning)
  * [Governance](#governance)
* Engine and Support Libraries
  * [kay](https://github.com/aeickhoff/kay) - enabling distributed simulation & multiplayer
  * [descartes](https://github.com/aeickhoff/descartes) - computational geometry and GIS algorithms

## Inspiration

* Games
  * [SimCity Series](https://en.wikipedia.org/wiki/SimCity)
  * [SimAnt](https://en.wikipedia.org/wiki/SimAnt)
  * [LEGO Loco](https://www.old-games.com/download/6261/lego-loco)
  * [A-Train (Take the A-Train III)](https://en.wikipedia.org/wiki/A-Train#A-Train_III)
  * [Rollercoaster Tycoon Series](https://en.wikipedia.org/wiki/RollerCoaster_Tycoon)
  * [Factorio](https://en.wikipedia.org/wiki/Factorio)
* Books/Ideas
  * [A Pattern Language](https://en.wikipedia.org/wiki/A_Pattern_Language) (Christopher Alexander)

## Philosophy

* Citybound is a game about:
  * **The beauty of emergent complexity,** in the familiar example of a city. (`EMERGENCE`)
  * **The challenge of understanding `INTERDEPENDENT SYSTEMS`,** that form a whole.
  * **The experience of `PLANNING`,** i.e. having a complex idea, developing it, carefully preparing it and making it real.
  * **The experience of `COLLABORATION`,** the way humans work together to achieve great things.
  * The interplay between the **player's conscious action and the cities' organic reaction** (`CITY AS AN ORGANISM`)
* Citybound aspires to:
  * `INTERACTION BY PAINTING`, i.e. through powerful but simple tools with haptic qualities
  * `CLARITY OF CAUSE AND EFFECT`, making patterns of behavior and consequences of one’s actions obvious- in the small and in the large

## Gameplay Decisions

* Cities are built in large continuous regions with several million inhabitants (`VASTNESS`)
* Time progresses in `OVERLAID TIME-SCALES`: Daily and yearly activities happen at a similar pace. (`MULTI-SCALE PHENOMENA`)
* Cities consist of persistent `UNIQUE INDIVIDUALS`, i.e. people & businesses

## Implementation Philosophy

* Develop systems from first principles, with complex behaviors emerging from simple microscopic interactions `(emergence)`
* Solve problems generally, do not restrict thinking in specialized problem instances `(actual problem)`
* Be brave to do things in new ways, where necessary `(radicality)`

## Implementation Decisions

* Use Rust as the main programming language for high performance and a strong type system to rely on
* Use an actor-based model for isolation, dynamic dispatch and simple parallelization and networking
* Use a Rust & JavaScript combination to implement a cross-platform, WebGL based UI that communicates with a simulation backend



## Hierarchy of Player Skills

* "Building a city"
  * "Observation & Inspection"
    * ~~["Reading Statistics & Maps"](inspection/stats/README.md)~~
    * ~~["Inspecting Individual Households"](inspection/households/README.md)~~
    * ~~["Reading Stories"](inspection/stories/README.md)~~
  * "Planning"
    * ["Transport Infrastructure Planning"](transport/planning/README.md)
    * ~~["Utility Infrastructure Planning"](utilities/README.md)~~
    * ~~["Zoning"](zoning/README.md)~~
    * ~~["Services Planning"](services/README.md)~~
    * ~~["Terraforming"](environment/terraforming/README.md)~~
  * "Execution & Finances"
    * ~~["Projects & Grants"](projects/README.md)~~
    * ~~["Budgeting"](finances/README.md)~~
    * ~~["Taxation"](finances/README.md)~~

# Simulation Core

The simulation core takes care of the progression and scales of in-game time by frequently updating continuous processes as well as sending one-off notifications and timeouts

## Prior Art & Inspiration

## Inspiration

* Compressed and overlaid timescales to make the player experience short-term and long-term phenomena at once `(multi-scale)`

## Decisions

## Implementation Philosophy

## Implementation Decisions

# Transport

## Subtopics

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

## Philosophy

* Traffic is the pulse of a city `(city = organism)`
* Traffic is a chaotic emergent phenomenon (`EMERGENCE`)
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

## Gameplay & Skills

* "Building a city"
   * "Planning"
      * "Transport Infrastructure Planning"
         * ["Road Planning"](planning/README.md#road-planning-skill)
         * ~~["Rail & Tram Planning"]()~~
         * ~~["Channel/Waterway Planning"]()~~
         * ~~["Airport Planning"]()~~
         * ~~["Public Transport Planning"]()~~

# Car Microtraffic

## Philosophy & Decisions

* See [Transport](../README.md)

## Implementation Philosophy

* Keep car interactions as local as possible computationally

## Implementation Decisions

* **Cars themselves are not actors, one lane is the atomic actor,** it updates all the cars on it in one go
* Cars mostly move along a lane like a train, only on switch lanes they can move sideways to merge between normal lanes
* Cars react to general obstacles, which are either the next car on their own lane, or cars on interacting lanes, whose position is mapped onto the current lane
    * Interactions are distinguished as parallel/opposing/merging
    * Interacting lanes after each update exchange their cars as obstacles for the other lane
* Use well-known 1D acceleration and breaking model and extend it to multi-lane behaviour
   * "Intelligent Driver Model", see [intelligent_acceleration.rs](./intelligent_acceleration.rs)
* Cars on switch lanes have more complex behaviour, reacting to cars on their left & right lanes and other cars merging between them.
    * Based on the perceived risk/required breaking, merging cars can decide to abort their merge
* Also see [Road Lanes](../lane/README.md)


# Road Lanes

## Philosophy

* The complex fabric of real road geometry goes beyond "streets" and "intersections", which at best can be understood as loose groups of related lanes (`EMERGENCE`, `ACTUAL PROBLEM`)

## Design Decision

* The atomic unit of road geometry in Citybound is the individual lane.
* Roads and intersections are just collections of lanes `(emergence)` `(flexibility)`

## Implementation Philosophy

* Really make individual lanes the main abstraction, only group them into higher abstractions when absolutely necessary.
* If possible, only handle a group of "nearby/related" lanes temporarily while using a tool
* Also see [Road Planning](../planning/README.md)

## Implementation Decisions

* Each lane keeps track of
   * it's connectivity to preceding/following lanes (connected to it's start or end)
   * lanes it overlaps, distinguishing between nearly-parallel overlaps and opposing overlaps
* Special case: two parallel lines that can be merged between are not directly connected
   * instead, an invisible "switch lane" is placed along their range of overlap
   * the lanes on either side have a special parallel overlap relation ship to the switch lane only, not to each other.
   * the reason for this is that merging car behaviour is a more complex special case
* Also see [Car Microtraffic](../microtraffic/README.md)

# Economy & Household Behaviour

## Prior Art & Inspiration

* [Prof. Philip Mirowski: Should Economists be Experts in Markets or Experts in "Human Nature"?](https://www.youtube.com/watch?v=xfbVPDNl7V4)
* Roller Coaster Tycoon as the gold standard for individual, grouped, localised thoughts as a key tool to inform game decisions

## Philosophy

* All macro-economic behaviours arise from interactions of individual households (families, companies) `(emergence)`
* Members of households have both selfish and shared goals, resulting in a trade-off between individuality and cooperation
* Locality of households and interactions is essential and enables all kinds of logistics and distributed industries
   * All resources that have to physically moved in real life have to also be moved in Citybound

* The real estate market is a huge factor in shaping a city

* The character of a city is largely determined by the sum of its industries
* The vast majority of consumed resources and services depend on complex chains of earlier resources, these resource chains/trees are at the core of shaping industries, the required logistics and thus also a city
* The player is not supposed to manage supply/demand/resource chains, but to enable them to be as effective as possible, through the means of infrastructure planning and zoning/governance

* Even though the amount of information presented in the game might go way beyond realistically knowable things, this contrast in availability of information compared to the real world has an educational and almost call-to-action aspect to it

* Individuals act as a lens to see the game world from, other than the players birds-eye-view
* Overall "macro" appearance of the city hints at general issues which might inspire you to investigate in detail

## Decisions

* Provide both omniscient-level information about individuals as well as realistic large-scale indicators
* Actually model the real estate market with ownership and businesses, including city government ownership
* All complex resources can only be a) traded or b) created from its component parts, never "out of nowhere"

## Implementation Philosophy

* Having to rely on absolute amounts for resources makes balancing very hard and amplifies bugs and makes the system potentially very unstable
* The amount of different resource types and how fine-grained they are is to be chosen as minimal as possible, while still producing interesting and prevalent resource chains

## Implementation Decisions

* Resources represent both tangible goods as well as intangible (psychological, physiological, social) metrics
* All of these are measured using scalar values, from -infinity to infinity
   * these values represent how well a household is doing on a particular resource (0 representing ideal), not necessarily absolute stockpile values
* One household and its associated resources are always tied to one specific building/location - if this is one location of a larger company, these cooperate mostly as if they were separate businesses
* "Crafting recipes" to determine both how resource combine, at which costs, and which businesses do that

## Parts

* Households (20% alpha)
    * [Families & Persons](./households/family)
    * ~~[Businesses / Industries]()~~
    * ~~[Neighboring Cities]()~~
* Markets
    * ~~[Market for Goods]()~~
    * ~~[Real Estate Market]()~~
    
## Skills

* "Building a city"
  * "Observation & Inspection"
    * "Inspecting Resource Chains"
      * Godmode-style knowledge
        * "Inspect all consumers of the products of a business"
        * "Inspect all the source producers of input resources of a business"
        * "Inspect the full journey/lifecycle of an end product, including becoming waste/..."
        * "Business surplus/deficits per resource category"
        * "Infer bottlenecks/problems by showing all related "nodes" in the economic network of one particular business, including paths"
      * Realistic knowledge
        * "Infer industry composition based on evident architectural styles"
        * "Directly show all businesses of a precise resource category"
        * "Infer causes of homelessness"
        * "Learn from protests"
        * "Learn from topical advisors" ?
        * "Learn about a neighborhood by looking at it"
  * "Planning"
  * "Execution & Finances"

# Governance

## Prior Art & Inspiration

## Philosophy

## Decisions

## Implementation Philosophy

## Implementation Decisions

## Parts

* ~~[Zoning & Spatial Decrees]()~~
* ~~[Taxation & Budgeting]()~~

# Planning

## Prior Art & Inspiration

* Code version control systems and file difference visualizers
* Blueprints/ghost parts in Factorio

## Philosophy

* In reality it is not only physically impossible to instantaneously build infrastructure, but there are good reasons why planning is employed before even deciding what to build `(planning)`
  * Plans are easy to revise and iterate on, changing existing structure is extremely costly
  * You can start planning something even when you do not have the permission or resources to implement the plan yet. In fact, you might need to have a plan to show in order to acquire the necessary permission and resources
  * Planning should be as flexible and forgiving as possible

## Decisions

* The player only ever interacts directly with plans. The only way to affect construction of infrastructure is through implementation of plans.
* Plans have full undo/redo history.
* Plans clearly show structures to be added and to be removed relative to what exists. `(clarity)`
* Modifications of existing structures seamlessly become part of a plan, this should feel as tangible as creating new structures.

## Implementation Philosophy

* "Materialized Reality" and plans exists in completely different "spheres"

## Implementation Decisions

* Instead of keeping a perfect, complicated two-way mapping between plans and implemented reality at all times, they are kept separate
* When new structures are added to a plan or previously planned and implemented structures are modified within the plan, they are compared geometrically in detail with the materialized reality to calculate the actual diff of what would change when implementing the new plan

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

# Road Planning

## Prior Art & Inspiration

## Philosophy

* [Planning in general](../../planning/README.md)
* [Lanes as the atomic unit of roads & intersections](../lane/README.md)

## Decisions


## Implementation Philosophy


## Implementation Decisions


# "Road Planning" (Skill)

## Actions -> Effects

## Effects -> Feedback

## Feedback -> Learning

# Environment

## Prior Art & Inspiration

## Philosophy

## Decisions

## Implementation Philosophy

## Implementation Decisions

## Parts

* ~~[Terrain]()~~
* ~~[Water & Weather]()~~
* ~~[Vegetation & Pollution]()~~
* ~~[Culture & Political Context]()~~

# Families & Persons

## Philosophy

* Not only the exchange of physical goods influences the decision-making of families, but intangible psychological, physiological, social metrics play an equally if not more important role
* There is no central happiness metric, but only individual diverse motivations that vary both over time as well as from person to person - one can only get a rough overview of "how well someone is doing" by looking at all of them