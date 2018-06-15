# Plant Seeds (Species & Growth Models)

## Prior Art & Inspiration

### Life

* [Phyla](https://en.wikipedia.org/wiki/Phylum) (Wikipedia)
  * Life Taxonomic rank below Kingdom
* [Plant divisions](https://en.wikipedia.org/wiki/Category:Plant_divisions) article category (Wikipedia)
* [Gymnosperm](https://en.wikipedia.org/wiki/Gymnosperm) (Wikipedia)
  * A plant with unenclosed, "naked" seeds

### Technology

* [Simulated growth of plants](https://en.wikipedia.org/wiki/Simulated_growth_of_plants) (Wikipedia)
  * Includes references to many extant software solutions to model plant growth
* [L-system branching](http://www.mizuno.org/applet/branching/)
* [/r/proceduralgeneration](https://www.reddit.com/r/proceduralgeneration/search?q=trees&restrict_sr=on)
  * Search for "trees"
* [IXORA](http://www.ixora.org/) (Private company)
  * Tensor-based graph computations
* [TensorFlow Rust language bindings](https://github.com/tensorflow/rust)
* [Algorithmic Botany](http://algorithmicbotany.org/papers/) papers (University of Calgary)

## Philosophy

### Trees
* Woody Trunks
* Woody Branches
* Stems
* Leaves

See [Lifecycle of a Tree](https://www.woodlandtrust.org.uk/blog/2017/06/life-cycle-of-a-tree/)

#### Evergreens

Evergreen trees do not undergo [Abscission](../lifecycle#dormancy) (Dormancy) during their lifecycle

##### Conifers

Cone-bearing trees usually with pine needles.

##### Broadleaf

See [12 Broadleaf Evergreen Street Trees](https://www.portlandoregon.gov/parks/article/514100) for a list of broadleaf evergreen trees

### Bushes
* Stems
* Leaves
* "Bushy-ness" factor (i.e. How voluptuous is this bush?)
* Shapes
  * Circular/Spherical
  * Ovular (Ovular-prism?)
  * Organic irregularities
  * Manicured
    * **These cost money from the owner! (public or private)**

#### Shrubs

Shrubs are bushes with Woody Branches and are usually smaller than a tree. Shrubs also have thicker foliage than that of a Bush.

### Flowers
* Stems
* Leaves
* Flowers
* Flowering bushes
  * Flowering from stems growing on branches

### Questions

* Should we model most of the [Kingdom of Plantae](https://en.wikipedia.org/wiki/Plant)? 💥
* Can an organic bush become manicured?

## Decisions

* Graph-based L-system?
* Machine Learning, an [IXORA](http://www.ixora.org/)-esque tensor-based graph analog?

## Implementation Philosophy

### Parametric L-system Notation

* **Variables** : `A`, `B`, `Name`, `Seed`, `Branch`, etc.; Nodes in a plant's L-system tree
* **Constants** : `->`, `[`, `]`
    * `->` : Production
    * `[` : Denotes the beginning of a subtree for a preceding variable
    * `]` : Close a subtree
* **Production** : `A -> B`
    * `A` transforms into `B`
    * Meaning the tree on the left-hand-side is replaced with the right-hand-side

### Structures

#### Node thing

* Properties via `::std::ops::Deref` and `::std::ops::DerefMut` implementations? ([/r/rust Struct Composition vs Inheritance](https://www.reddit.com/r/rust/comments/51f60f/struct_composition_vs_inheritance/))
* `children: Vec[Node impls]` via 
* `parent: Node` reference?
* `impl std::iter::Iterator` (iterate over all children nodes, depth first traversal)
* `age: uint`
* `fn grow(&self, amount: uint = 1) -> Fate::Live | Fate::Die`
  * Default impl: `self.age += amount`

* A node's age is used in the [Plant Renderer](../rendering) as a measure of the node's length.
  * Higher the age, longer the plant node
  * Higher the age, larger the tapered radius of the node also?

* Plant
  * `impl Node`
  
#### Plant Parts

All parts `impl Node`

* Seed
* Sprout
* Root
* Shoot
  * Whip (`Shoot-Whip`)
  * Dicot (`Shoot-Dicot`)
* Trunk
* Branch
  * Root (`Branch-Root`)
  * Wood (`Branch-Wood`)
  * Green (`Branch-Green`)
* Stem
* Leaf
  * Every `Leaf` has a `Stem`, but a `Stem` can have more than one `Leaf`
  
**Questions**

* Is a `Trunk` simply a `Branch` with only one child?
  * **No**, a tree's trunk can have branches and still have a trunk continue skywards
    * So `Trunk` nodes can have child `Trunk` nodes

### Traits, i.e. "is-a"

#### Plant Types

* Tree
* Bush
* Deciduous
  * All nodes without this trait are evergreen
* Flower

### Growth L-system

1. Seed
  * Begins as a lone seed, usually dried
2. Germination
  * `Seed -> Sprout`
  * Germinates after planting with adequate water, food, and maximum distance to surface
3. Sprout
  * `1. Sprout -> Root [ Shoot-Whip ]`
  * `2. Sprout -> Root [ Shoot-Dicot ]`
  * Becomes a rooted seedling when sprout breaks through soil's surface
  * If the plant is a dicot then the seedling will have two leaves, otherwise it's a whip
4. Seedling
  * `Root [ Shoot-Whip ] -> [Root Root] Branch-Root [ Trunk [ Branch-Green [ Stem [ Leaf ] Shoot-Whip Stem [ Leaf ] ] Branch-Green [ Stem [ Leaf ] Shoot-Whip Stem [ Leaf ] ] ] ]`
  * Becomes a Sapling
5. Sapling
  * **TODO:** L-System Productions
  * Branching root structure
  * Smooth bark on a woody trunk
  * "Symmetric" green branches
  * "Symmetric" stems with leaves surrounding green shoots
6. Primary growth
  * **TODO:** L-System Productions
  * More root branches
  * Rough bark trunk
  * Less rough bark branches
  * Green branches, stems, shoots, and leaves
7. Adult Lifecycle
  * **TODO:** L-System Productions
  * Reproductive leaf nodes

## Implementation Decisions
