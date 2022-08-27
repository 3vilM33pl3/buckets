package buckets

import (
	"fmt"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
	"os"
)

// createCmd represents the create command
var renameCmd = &cobra.Command{
	Use:   "rename [name]",
	Short: "Rename existing bucket",
	Long: `Renames an existing bucket.

Example:
bucket rename [old name] [new name]

`,
	Run: func(cmd *cobra.Command, args []string) {

		// Check if the command has the correct number of arguments
		if len(args) == 0 {
			fmt.Println("Please provide a old and new bucket name")
			os.Exit(1)
		} else if len(args) == 1 {
			fmt.Println("Please provide a new bucket name")
			os.Exit(1)
		}

		// Check if directory exists. If it does, then error.
		if _, err := os.Stat(args[0]); os.IsNotExist(err) {
			fmt.Printf("Bucket doesn't exists in this directory: %s\n", args[0])
			os.Exit(1)
		}

		// Check if .b directory exists in bucket. If it does, then error
		if _, err := os.Stat(args[0] + "/.b"); os.IsNotExist(err) {
			fmt.Printf("Directory isn't a bucket, missing .b directory\n")
			os.Exit(1)
		} else {
			err := os.Rename(args[0], args[1])
			if err != nil {
				fmt.Println(err)
				os.Exit(1)
			}
			viper.SetConfigName("config")
			viper.SetConfigType("yaml")
			viper.AddConfigPath("./" + args[1] + "/.b/")
			err = viper.ReadInConfig()
			if err != nil {
				fmt.Println(err)
				os.Exit(1)
			}

			viper.Set("name", args[1])
			viper.WriteConfig()

		}

		// Create the directory/bucket

	},
}

func init() {
	rootCmd.AddCommand(renameCmd)
}
