# Economy & Household Behaviour

## Prior Art & Inspiration

* [Prof. Philip Mirowski: Should Economists be Experts in Markets or Experts in "Human Nature"?](https://www.youtube.com/watch?v=xfbVPDNl7V4)

## Philosophy

* All macro-economic behaviours arise from interactions of individual households (families, companies) `(emergence)`
* Members of households have both selfish and shared goals, resulting in a tradeoff between individuality and cooperation
* Have local effects and importance of location, enabling all kinds of logistics and distributed industries
   * All resources that have to physically moved in real life have to also be moved in Citybound
* The real estate market is a huge factor in shaping a city
* Not only the exchange of physical good influences the decisionmaking of households, but intagible phsychological, physiological, social metrics play an equally if not more important role
* The vast majority of consumed resources and services depend on complex chains of earlier resources, these resource chains/trees are at the core of shaping industries, the required logistics and thus also a city

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
