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

