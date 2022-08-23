package buckets

import (
	"fmt"
	"github.com/spf13/cobra"
	"os"
)

// createCmd represents the create command
var createCmd = &cobra.Command{
	Use:   "create [name]",
	Short: "Create a new bucket for content",
	Long: ` Create a new bucket for storing content.


`,
	Run: func(cmd *cobra.Command, args []string) {

		// Check if the command has the correct number of arguments
		if len(args) != 1 {
			fmt.Println("Please provide a bucket name")
			os.Exit(1)
		}

		// Check if directory/bucket exists. If it does, then error.
		if _, err := os.Stat(args[0]); !os.IsNotExist(err) {
			fmt.Println("Bucket already exists")
			os.Exit(1)
		} else {
			os.Mkdir(args[0], 0755)
			os.Mkdir(args[0]+"/.b", 0755)

			config := []byte(`config for bucket: ` + args[0])
			os.WriteFile(args[0]+"/.b/config", config, 0644)

		}

		// Create the directory/bucket

	},
}

func init() {
	rootCmd.AddCommand(createCmd)
}
