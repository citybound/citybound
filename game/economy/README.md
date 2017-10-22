# Economy & Household Behaviour

## Prior Art & Inspiration

* [Prof. Philip Mirowski: Should Economists be Experts in Markets or Experts in "Human Nature"?](https://www.youtube.com/watch?v=xfbVPDNl7V4)
* Roller Coaster Tycoon as the gold standard for individual, grouped, localised thoughts as a key tool to inform game decisions

## Philosophy

* All macro-economic behaviours arise from interactions of individual households (families, companies) `(emergence)`
* Members of households have both selfish and shared goals, resulting in a tradeoff between individuality and cooperation
* Have local effects and importance of location, enabling all kinds of logistics and distributed industries
   * All resources that have to physically moved in real life have to also be moved in Citybound
* The real estate market is a huge factor in shaping a city
* Not only the exchange of physical good influences the decisionmaking of households, but intagible phsychological, physiological, social metrics play an equally if not more important role
* The vast majority of consumed resources and services depend on complex chains of earlier resources, these resource chains/trees are at the core of shaping industries, the required logistics and thus also a city
* The character of a city is largely determined by the sum of its industries
* The player is not supposed to manage supply/demand/resource chains, but to enable them to be as effective as possible, thorugh the means of infrastructure planning and zoning/governance
* There is no central happiness metric, but only individual diverse motivations that vary both over time as well as from person to person - one can only get a rough overview of "how well someone is doing" by looking at all of them
* Even though the amount of information presented in the game might go way beyond realisticly knowable things, this contrast in availabiltiy of information compared to the real world has an educational and almost call-to-action aspect to it
* Following actual individuals mostly provides "story/worldbuildings", decisions are made based on aggregates?
* Individuals act as a lens to see the game world from, other than the players birds-eye-view
* Overall "macro" appearance of the city hints at general issues which might inspire you to investigate in detail

## Decisions
* Actually model the real estate market with ownership and businesses, including city government ownership
* All complex resources can only be a) traded or b) created from its component parts, never "out of nowhere"

## Implementation Philosophy
* Having to rely on absolute values makes balancing very hard and amplifies bugs and makes the system potentially very unstable

## Implementation Decisions

* Resources represent both tangible goods as well as intangible (phsychological, physiological, social) metrics
* All of these are measured using scalar values, from -infinitiy to infinity
   * these values represent how well a household is doing on a particular resource (0 representing ideal), not necessarily absolute stockpile values
* One houshold and its associated resources are always tied to one specific building/location - if this is one location of a larger company, there needs to be some way of cooperation between different locations ❓
* "Crafting recipies" to determine both how resource combine, at which costs, and which businesses do that

## Parts

* Households (20% alpha)
    * ~~[Families]()~~
    * ~~[Businesses]()~~
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
      * Realistic knowledge
        * "Infer industry composition based on evident architectural styles"
        * "Directly show all businesses of a precise resource category"
        * "Infer bottlenecks/problems by showing all related "nodes" in the economic network of one particular business, including paths"
        * "Infer causes of homelessness"
        * "Learn from protests"
        * "Learn from topical advisors" ?
        * "Learn about a neighborhood by looking at it"
  * "Planning"
  * "Execution & Finances"

