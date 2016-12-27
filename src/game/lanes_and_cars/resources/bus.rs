//          a simple bus
//
//      .  6--------------------------7
//  1--------------------------2   `   \     2.5
//  |      |                    \       8     |
//  |      |                     3   `  |     |
//  |      |                     |      |     Z
//  |   .  5- - - - - - - - - - -|------9     |    Y   .  0.9
//  0----------------------------4   `        0    -0.9
//
// -3 ---------------X---------- 3

use ::monet::Vertex;

pub fn create() -> ::monet::Thing {
    ::monet::Thing::new(
        vec![                      //X,     Y,    Z
            Vertex { position: [ -3.00,  -0.9, 0.00 ] }, // 0
            Vertex { position: [ -3.00,  -0.9, 2.50 ] }, // 1
            Vertex { position: [  2.33,  -0.9, 2.50 ] }, // 2
            Vertex { position: [  3.00,  -0.9, 1.33 ] }, // 3
            Vertex { position: [  3.00,  -0.9, 0.00 ] }, // 4
            
            Vertex { position: [ -3.00,   0.9, 0.00 ] }, // 5
            Vertex { position: [ -3.00,   0.9, 2.50 ] }, // 6
            Vertex { position: [  2.33,   0.9, 2.50 ] }, // 7
            Vertex { position: [  3.00,   0.9, 1.33 ] }, // 8
            Vertex { position: [  3.00,   0.9, 0.00 ] }, // 9
        ],
        vec![
            // right side
            0, 1, 2,
            0, 2, 4,
            2, 3, 4,
            // left side
            5, 6, 7,
            5, 7, 9,
            7, 8, 9,
            // connection between sides (front to back)
            0, 1, 5,
            1, 5, 6,
            1, 6, 7,
            1, 2, 7,
            2, 7, 8,
            2, 3, 8,
            3, 8, 9,
            3, 4, 9,
            0, 4, 5,
            5, 9, 4,
        ]
    )
}
