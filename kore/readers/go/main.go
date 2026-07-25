package main

import (
	"fmt"
	"os"
	"kore_reader/kore"
)

func main() {
	path := "../../test/test_v2.kore"
	if len(os.Args) > 1 { path = os.Args[1] }

	f, err := kore.Open(path)
	if err != nil { fmt.Fprintf(os.Stderr, "Error: %v\n", err); os.Exit(1) }

	fmt.Println(f.Info())
	for _, col := range f.Columns {
		fmt.Printf("\n%s (%s):\n", col.Name, col.TypeName())
		switch col.KType {
		case kore.KTypeInt:
			vals, _ := f.ReadIntColumn(col.Name)
			for i := 0; i < len(vals) && i < 10; i++ { fmt.Printf("  [%d] %d\n", i, vals[i]) }
		case kore.KTypeFloat:
			vals, _ := f.ReadFloatColumn(col.Name)
			for i := 0; i < len(vals) && i < 10; i++ { fmt.Printf("  [%d] %.4f\n", i, vals[i]) }
		case kore.KTypeBool:
			vals, _ := f.ReadBoolColumn(col.Name)
			for i := 0; i < len(vals) && i < 10; i++ { fmt.Printf("  [%d] %v\n", i, vals[i]) }
		case kore.KTypeStr:
			vals, _ := f.ReadStrColumn(col.Name)
			for i := 0; i < len(vals) && i < 10; i++ { fmt.Printf("  [%d] \"%s\"\n", i, vals[i]) }
		}
	}
	fmt.Println("\nDONE")
}
