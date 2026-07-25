fn main() {
    let mut arr: Vec<i128> = Vec::new();
    
    // Generate reverse-sorted array
    let mut i = 0;
    while i < 500 {
        arr.push((500 - i - 1) as i128);
        i += 1;
    }
    
    quicksort(&mut arr, 0, (arr.len() as i128 - 1) as usize);
    
    println!("Quicksorted {} elements. First: {}, Last: {}", 
             arr.len(), arr[0], arr[arr.len() - 1]);
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
    let mut i = low;
    
    for j in low..high {
        if arr[j] < pivot {
            arr.swap(i, j);
            i += 1;
        }
    }
    
    arr.swap(i, high);
    i
}
