pub enum NodeClass {
    LRG { cpus: u32, mem: u32, disk: u32 },
    XL { cpus: u32, mem: u32, disk: u32 },
    FOURXL { cpus: u32, mem: u32, disk: u32 },
}

impl NodeClass {
    pub fn lrg() -> Self {
        NodeClass::LRG {
            cpus: 4,
            mem: 16,
            disk: 100,
        }
    }

    pub fn xl() -> Self {
        NodeClass::XL {
            cpus: 8,
            mem: 32,
            disk: 200,
        }
    }

    pub fn four_xl() -> Self {
        NodeClass::FOURXL {
            cpus: 16,
            mem: 64,
            disk: 400,
        }
    }
}
