//          a simple ute
//
//                  .  B---------C
//              3---------4   `   \           1.65
//      .  9----| - .  A   \    .  D---E      |
//  1-----------2           5---6       \     Z
//  |   .  8 - - - - - - - - - - \-------F    |    Y   .  0.9
//  0----------------------------7    `       0    -0.9
//
// -2.25-----------X----------2.25

use monet::Vertex;

pub fn create() -> ::monet::Thing {
    ::monet::Thing::new(
        vec![
            Vertex { position: [-2.25, -0.9, 0.00] }, // 0
            Vertex { position: [-2.25, -0.9, 0.80] }, // 1
            Vertex { position: [-0.50, -0.9, 0.80] }, // 2
            Vertex { position: [-0.50, -0.9, 1.65] }, // 3
            Vertex { position: [1.25, -0.9, 1.65] }, // 4
            Vertex { position: [1.50, -0.9, 0.80] }, // 5
            Vertex { position: [1.85, -0.9, 0.80] }, // 6
            Vertex { position: [2.25, -0.9, 0.00] }, // 7

            Vertex { position: [-2.25, 0.9, 0.00] }, // 8
            Vertex { position: [-2.25, 0.9, 0.80] }, // 9
            Vertex { position: [-0.50, 0.9, 0.80] }, // A
            Vertex { position: [-0.50, 0.9, 1.65] }, // B
            Vertex { position: [1.25, 0.9, 1.65] }, // C
            Vertex { position: [1.50, 0.9, 0.80] }, // D
            Vertex { position: [1.85, 0.9, 0.80] }, // E
            Vertex { position: [2.25, 0.9, 0.00] }, // F
        ],
        vec![
            // right side
            0, 1, 2,
            0, 2, 5,
            0, 5, 7,
            5, 6, 7,
            2, 3, 4,
            2, 4, 5,

            // left side
            8, 9, 0xA,
            8, 0xA, 0xD,
            8, 0xD, 0xF,
            0xD, 0xE, 0xF,
            0xA, 0xB, 0xC,
            0xA, 0xC, 0xD,

            // connection between sides (front to back)
            8, 9, 1,
            8, 1, 0,
            9, 0xA, 2,
            9, 2, 1,
            0xA, 0xB, 3,
            0xA, 3, 2,
            0xB, 0xC, 4,
            0xB, 4, 3,
            0xC, 0xD, 5,
            0xC, 5, 4,
            0xD, 0xE, 6,
            0xD, 6, 5,
            0xE, 0xF, 7,
            0xE, 7, 6u16
        ]
    )
}
