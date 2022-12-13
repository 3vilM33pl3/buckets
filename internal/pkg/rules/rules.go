package rules

import (
	"fmt"
	"github.com/mitchellh/hashstructure/v2"
	"github.com/spf13/viper"
	"os"
)

type Rule struct {
	Type  string
	Name  string
	Exist bool
}

func NewRule(itemType, name string) Rule {
	return Rule{itemType, name, true}
}

func (r *Rule) SetExist() {
	r.Exist = true
}

func (r *Rule) SetNotExist() {
	r.Exist = false
}

func (r *Rule) Save() {

	hash, err := hashstructure.Hash(r, hashstructure.FormatV2, nil)
	if err != nil {
		fmt.Println(err)
		os.Exit(1)
	}

	hashString := fmt.Sprintf("%d.yaml", hash)

	os.Chdir(".b")
	if _, err := os.Stat(hashString); !os.IsNotExist(err) {
		fmt.Println("Rule already exists")
		os.Exit(1)
	} else {
		viper.SetConfigType("yaml")
		viper.Set("name", r.Name)
		viper.Set("type", r.Type)
		viper.Set("exist", r.Exist)
		err := viper.SafeWriteConfigAs(hashString)
		if err != nil {
			fmt.Println(err)
			os.Exit(1)
		}

	}

}

const colorRed = "\033[0;31m "
const colorNone = "\033[0m"

func (r *Rule) Check() bool {
	if _, err := os.Stat(r.Name); os.IsNotExist(err) {
		fmt.Printf(colorRed+"Bucket '%s' does not exist\n "+colorNone, r.Name)
	}
	return false
}
