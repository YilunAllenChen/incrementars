use incrementars::prelude::{Incrementars, Observable};

pub fn main() {
    let mut dag = Incrementars::new();
    let length = dag.var(2.0);
    let area = dag.map(&length, |x| {
        println!("calculating area");
        x * x
    });

    // on initial stabilization, area is calculated to be 4.
    assert_eq!(area.observe(), 4.0);
    length.set(3.0);

    // right after setting, dag isn't stabilized yet.
    assert_eq!(area.observe(), 4.0);

    dag.stabilize();
    assert_eq!(area.observe(), 9.0);

    println!("introducing height...");
    let height = dag.var(5.0);
    let volume = dag.map2(&area, &height, |x, y| {
        println!("calculating volume");
        x * y
    });

    assert_eq!(volume.observe(), 45.0);

    println!("setting height (this shouldn't trigger area calculation!)");
    height.set(10.0);
    dag.stabilize();
    assert_eq!(volume.observe(), 90.0);

    println!("setting length (this should trigger area calculation)");
    length.set(2.0);
    dag.stabilize();
    assert_eq!(volume.observe(), 40.0);
}
