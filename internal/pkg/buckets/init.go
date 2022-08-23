package buckets

import (
	"fmt"
	"github.com/spf13/cobra"
	"os"
)

// initCmd represents the init command
var initCmd = &cobra.Command{
	Use:   "init",
	Short: "Initialize a new bucket repository",
	Long: `Initialize a new bucket repository. 

Creates a top-level .buckets directory and stores the configuration in it.

Example:
bucket init [repository name]
This will create a new repository with the name [repository name] in a directory with the same name.
`,
	Run: func(cmd *cobra.Command, args []string) {
		// Check if the command has the correct number of arguments
		if len(args) != 1 {
			fmt.Println("Please provide a repository name")
			os.Exit(1)
		}

		// First check if directory exists. If it does, then error.
		// If it doesn't, then create it with a .buckets directory in it
		if _, err := os.Stat(args[0]); !os.IsNotExist(err) {
			fmt.Println("Directory already exists")
			os.Exit(1)
		} else {
			os.Mkdir(args[0], 0755)
			os.Mkdir(args[0]+"/.buckets", 0755)

			config := []byte(`top level directory of repository: ` + args[0])
			os.WriteFile(args[0]+"/.buckets/config", config, 0644)
		}
	},
}

func init() {
	rootCmd.AddCommand(initCmd)

}
