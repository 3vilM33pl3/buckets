package buckets

import (
	"buckets/internal/pkg/rules"
	"fmt"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
	"log"
	"os"
)

var checkCmd = &cobra.Command{
	Use:   "check",
	Short: "Checks all expectations of a bucket",
	Long:  `Checks all expectations of a bucket.`,
	Run: func(cmd *cobra.Command, args []string) {
		files, err := os.ReadDir(".b/")
		if err != nil {
			log.Fatal(err)
		}

		isCool := true
		for _, file := range files {
			if file.Name() == "config.yaml" {
				continue
			}
			file, err := os.Open(".b/" + file.Name())
			if err != nil {
				fmt.Println(err)
				return
			}

			viper.ReadConfig(file)
			rule := rules.NewRule(viper.GetString("type"), viper.GetString("name"))
			if !rule.Check() && isCool {
				isCool = false
			}
		}

		if isCool {
			fmt.Println("Bucket doesn't meet expectations")
		}
	},
}

func init() {
	rootCmd.AddCommand(checkCmd)

}
