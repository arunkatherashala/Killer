fn main() {
    let mut arr: Vec<i128> = Vec::new();
    for i in 0..500 {
        arr.push((500 - i) as i128);
    }
    
    quicksort(&mut arr, 0, arr.len() - 1);
    
    println!("Quicksorted {} elements. First: {}, Last: {}", arr.len(), arr[0], arr[arr.len()-1]);
}

fn quicksort(arr: &mut Vec<i128>, low: usize, high: usize) {
    if low < high {
        let pi = partition(arr, low, high);
        if pi > 0 {
            quicksort(arr, low, pi - 1);
        }
        quicksort(arr, pi + 1, high);
    }
}

fn partition(arr: &mut Vec<i128>, low: usize, high: usize) -> usize {
    let pivot = arr[high];
    let mut i = low as i32;
    
    for j in low..high {
        if arr[j] < pivot {
            arr.swap(i as usize, j);
            i += 1;
        }
    }
    
    arr.swap(i as usize, high);
    i as usize
}
