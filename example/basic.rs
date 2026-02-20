fn main() {
    let rt = Arc::new(Executor {
        queue: Mutex::new(VecDeque::new())
    });

    rt.spawn(async {
        println!("Hello from task 1");
    });

    rt.spawn(async {
        println!("Hello from task 2");
    });
    rt.run();

    println!("1");
    println!("2");
}