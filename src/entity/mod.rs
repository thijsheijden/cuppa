pub mod setting;

#[derive(Debug, Clone)]
pub struct Drink {
    pub id: &'static str,
    pub name: &'static str,
    pub caffeine_mg: i32,
}

pub const ESPRESSO: Drink = Drink {
    id: "espresso",
    name: "Espresso",
    caffeine_mg: 63,
};

pub const DOUBLE_ESPRESSO: Drink = Drink {
    id: "double_espresso",
    name: "Double Espresso",
    caffeine_mg: 126,
};

pub const CAPPUCCINO: Drink = Drink {
    id: "cappuccino",
    name: "Cappuccino",
    caffeine_mg: 63,
};

pub const LATTE: Drink = Drink {
    id: "latte",
    name: "Latte",
    caffeine_mg: 63,
};

pub const AMERICANO: Drink = Drink {
    id: "americano",
    name: "Americano",
    caffeine_mg: 63,
};

pub const FLAT_WHITE: Drink = Drink {
    id: "flat_white",
    name: "Flat White",
    caffeine_mg: 77,
};

pub const DRIP_COFFEE: Drink = Drink {
    id: "drip_coffee",
    name: "Drip Coffee",
    caffeine_mg: 95,
};

pub const COLD_BREW: Drink = Drink {
    id: "cold_brew",
    name: "Cold Brew",
    caffeine_mg: 200,
};

pub const POUR_OVER: Drink = Drink {
    id: "pour_over",
    name: "Pour Over",
    caffeine_mg: 95,
};

pub const MACCHIATO: Drink = Drink {
    id: "macchiato",
    name: "Macchiato",
    caffeine_mg: 63,
};

pub const MOCHA: Drink = Drink {
    id: "mocha",
    name: "Mocha",
    caffeine_mg: 63,
};

pub const RED_EYE: Drink = Drink {
    id: "red_eye",
    name: "Red Eye",
    caffeine_mg: 158,
};

pub const BLACK_TEA: Drink = Drink {
    id: "black_tea",
    name: "Black Tea",
    caffeine_mg: 47,
};

pub const GREEN_TEA: Drink = Drink {
    id: "green_tea",
    name: "Green Tea",
    caffeine_mg: 28,
};

pub const MATCHA: Drink = Drink {
    id: "matcha",
    name: "Matcha",
    caffeine_mg: 70,
};

pub const ALL: &[&'static Drink] = &[
    &ESPRESSO,
    &DOUBLE_ESPRESSO,
    &CAPPUCCINO,
    &LATTE,
    &AMERICANO,
    &FLAT_WHITE,
    &DRIP_COFFEE,
    &COLD_BREW,
    &POUR_OVER,
    &MACCHIATO,
    &MOCHA,
    &RED_EYE,
    &BLACK_TEA,
    &GREEN_TEA,
    &MATCHA,
];
