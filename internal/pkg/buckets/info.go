package buckets

import (
	"fmt"
	"github.com/spf13/cobra"
	"io/ioutil"
	"log"
	"os"
)

var infoCmd = &cobra.Command{
	Use:   "info",
	Short: "Get information about the repository",
	Long:  `Get information about the repository. Number of buckets, size of repository,`,

	Run: func(cmd *cobra.Command, args []string) {
		originalDir, _ := os.Getwd()
		for {
			currentDir, _ := os.Getwd()
			if _, err := os.Stat(currentDir + "/.buckets"); !os.IsNotExist(err) {
				fmt.Println("Found repository")
				break
			} else {
				topLevelDirCheck, _ := os.Getwd()

				if topLevelDirCheck == "/" || topLevelDirCheck[1:] == ":\\" {
					fmt.Println("Not a repository")
					os.Chdir(originalDir)
					os.Exit(1)
				}
			}
			os.Chdir("..")
		}

		os.Chdir(originalDir)
		content, err := ioutil.ReadFile(".buckets/config")

		if err != nil {
			os.Chdir(originalDir)
			log.Fatal(err)
		}

		fmt.Println(string(content))

	},
}

func init() {
	rootCmd.AddCommand(infoCmd)
}
