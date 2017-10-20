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

