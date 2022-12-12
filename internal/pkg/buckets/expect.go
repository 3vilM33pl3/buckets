package buckets

import (
	"buckets/internal/pkg/rules"
	"fmt"

	"github.com/spf13/cobra"
)

// expectCmd represents the expect command
var expectCmd = &cobra.Command{
	Use:   "expect",
	Short: "What should the bucket expect?",
	Long: `What should the bucket expect? This command will help you set up bucket rules
and expectations. 
Example: 'bucket expect bucket Flower' will set a rule to expect a bucket named Flower.
`,
	Run: func(cmd *cobra.Command, args []string) {
		if len(args) == 0 {
			fmt.Println("Not enough arguments")
		}

		if args[0] == "bucket" {
			rule := rules.NewRule("bucket", args[1])
			rule.Save()

		}

	},
}

func init() {
	rootCmd.AddCommand(expectCmd)
}
