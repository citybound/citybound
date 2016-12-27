//          a simple truck
//
//      .  8---------------------9
//  1---------------------2   `  A----B       2.5
//  |      |              3----4   `   \      |
//  |      |                    \       C     |
//  |      |                     5   `  |     Z
//  |   .  7- - - - - - - - - - -|------D     |    Y   .  0.9
//  0----------------------------6   `        0    -0.9
//
// -3 ---------------X---------- 3

use ::monet::Vertex;

pub fn create() -> ::monet::Thing {
    ::monet::Thing::new(
        vec![                      //X,     Y,    Z
            Vertex { position: [ -3.00,  -0.9, 0.00 ] }, // 0
            Vertex { position: [ -3.00,  -0.9, 2.50 ] }, // 1
            Vertex { position: [  1.50,  -0.9, 2.50 ] }, // 2
            Vertex { position: [  1.50,  -0.9, 2.00 ] }, // 3
            Vertex { position: [  2.66,  -0.9, 2.00 ] }, // 4
            Vertex { position: [  3.00,  -0.9, 1.00 ] }, // 5
            Vertex { position: [  3.00,  -0.9, 0.80 ] }, // 6
            
            Vertex { position: [ -3.00,   0.9, 0.00 ] }, // 7
            Vertex { position: [ -3.00,   0.9, 2.50 ] }, // 8
            Vertex { position: [  1.50,   0.9, 2.50 ] }, // 9
            Vertex { position: [  1.50,   0.9, 2.00 ] }, // A
            Vertex { position: [  2.66,   0.9, 2.00 ] }, // B
            Vertex { position: [  3.00,   0.9, 1.00 ] }, // C
            Vertex { position: [  3.00,   0.9, 0.80 ] }, // D
        ],
        vec![
            // right side
            0, 1, 3,
            1, 2, 3,
            0, 3, 6,
            3, 4, 5,
            3, 5, 6,
            // left side
            7, 8, 0xA,
            8, 9, 0xA,
            7, 0xA, 0xD,
            0xA, 0xB, 0xC,
            0xA, 0xC, 0xD,
            // connection between sides (front to back)
            0, 1, 7,
            1, 7, 8,
            1, 8, 2,
            9, 2, 8,
            0xA, 9, 2,
            0xA, 3, 2,
            0xA, 3, 4,
            0xB, 4, 0xA,
            4, 0xB, 5,
            0xC, 5, 0xB,
            0xD, 0xC, 5,
            0xD, 6, 5,
            0, 6, 7,
            0xD, 7, 6
        ]
    )
}
