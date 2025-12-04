package tree_sitter_rcss_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_rcss "github.com/nathan-king/rcss-preprocessor/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_rcss.Language())
	if language == nil {
		t.Errorf("Error loading RCSS grammar")
	}
}
