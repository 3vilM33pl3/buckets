package buckets

import (
	"fmt"
	"github.com/manifoldco/promptui"
	"github.com/spf13/cobra"
)

type ResourceType int

const (
	INPUT  ResourceType = iota
	CREATE              = iota
	OUTPUT              = iota
)

// setCmd represents the set command
var setCmd = &cobra.Command{
	Use:   "set",
	Short: "A brief description of your command",
	Long: `A longer description that spans multiple lines and likely contains examples
and usage of using your command. For example:

Cobra is a CLI library for Go that empowers applications.
This application is a tool to generate the needed files
to quickly create a Cobra application.`,
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("set called")

		prompt := promptui.Select{
			Label: "Select resource type",
			Items: []string{"Input", "Create", "Output"},
		}

		_, result, err := prompt.Run()

		if err != nil {
			fmt.Printf("Prompt failed %v\n", err)
			return
		}

		var resourceType ResourceType

		switch result {
		case "Input":
			resourceType = INPUT
		case "Create":
			resourceType = CREATE
		case "Output":
			resourceType = OUTPUT
		default:
			resourceType = INPUT
		}

		fmt.Printf("You choose %q\n", resourceType)
	},
}

func init() {
	expectCmd.AddCommand(setCmd)

	// Here you will define your flags and configuration settings.

	// Cobra supports Persistent Flags which will work for this command
	// and all subcommands, e.g.:
	// setCmd.PersistentFlags().String("foo", "", "A help for foo")

	// Cobra supports local flags which will only run when this command
	// is called directly, e.g.:
	// setCmd.Flags().BoolP("toggle", "t", false, "Help message for toggle")
}
