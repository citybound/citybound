//
//        5---------6
//      . |       . |
//    1---------2   |  2.0
//    |   |     |   |  |
//    |   |     |   |  |
//    |   |     |   |  |
//    |   |     |   |  |
//    |   |     |   |  Z
//    |   |     |   |  |
//    |   |     |   |  |
//    |   4-----|---7  |
//    | .       | .    |
//    0---------3      0.0
// -0.6----X----0.6

use ::monet::Vertex;

pub fn create() -> ::monet::Thing {
    ::monet::Thing::new(
        vec![
            Vertex { position: [-0.6, -0.6, 0.0] }, // 0
            Vertex { position: [-0.6, -0.6, 2.0] }, // 1
            Vertex { position: [-0.6,  0.6, 2.0] }, // 2
            Vertex { position: [-0.6,  0.6, 0.0] }, // 3

            Vertex { position: [ 0.4, -0.4, 0.0] }, // 4
            Vertex { position: [ 0.4, -0.4, 2.0] }, // 5
            Vertex { position: [ 0.4,  0.4, 2.0] }, // 6
            Vertex { position: [ 0.4,  0.4, 0.0] }, // 7
        ],
        vec![
            // front side
            0, 1, 2,
            0, 2, 3,
            // left side
            4, 5, 1,
            4, 1, 0,
            // back side
            7, 6, 5,
            7, 5, 4,
            // right side
            3, 2, 6,
            3, 6, 7,
            //top
            1, 5, 6,
            1, 6, 2
        ]
    )
}

pub fn create_light() -> ::monet::Thing {
    ::monet::Thing::new(
        vec![
            Vertex { position: [-0.7, -0.4, 0.0] }, // 0
            Vertex { position: [-0.7, -0.4, 0.6] }, // 1
            Vertex { position: [-0.7,  0.4, 0.6] }, // 2
            Vertex { position: [-0.7,  0.4, 0.0] }, // 3
        ],
        vec![
            0, 1, 2,
            0, 2, 3
        ]
    )
}

pub fn create_light_left() -> ::monet::Thing {
    ::monet::Thing::new(
        vec![
            Vertex { position: [-0.7,  0.4, 0.3] }, // 0
            Vertex { position: [-0.7, -0.4, 0.0] }, // 1
            Vertex { position: [-0.7, -0.4, 0.6] }, // 2
        ],
        vec![
            0, 1, 2
        ]
    )
}

pub fn create_light_right() -> ::monet::Thing {
    ::monet::Thing::new(
        vec![
            Vertex { position: [-0.7,  0.4, 0.0] }, // 0
            Vertex { position: [-0.7,  0.4, 0.6] }, // 1
            Vertex { position: [-0.7, -0.4, 0.3] }, // 2
        ],
        vec![
            0, 1, 2
        ]
    )
}