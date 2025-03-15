# Notex

自作マークアップ言語
@[](https://github.com/season1618/notex)

## 概要
文書、テンプレートHTML、出力ファイルを用意して実行。
```
$ <notex> <template>.html <source>.md (<destination>.html)
```

スタイルはCSSで指定する。

### 属性
文書から各種データを抽出しテンプレート中の`{属性名}`に埋め込む。利用可能なデータは以下の通り。
- `title`: h1タグ`#`の見出しを文書のタイトルとして用いる。
- `toc`: 文書中の見出しから目次を生成し順序付きリストとして表示。
- `year`, `month`, `day`, `hour`, `minute`, `second`: 文書をHTMLに変換した時刻。
- `content`: 本文。

### テンプレートの例
この文書のテンプレートを示す。

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <link rel="stylesheet" href="./index.css">
  <!-- 省略 -->
  <title>{title}</title>
</head>
<body>
  <nav id="toc">
    <h4><a href="#{title}">{title}</a></h4>
    {toc}
  </nav>
  <div id="content">
    <p style="text-align: right;">最終更新: {year}/{month}/{day} {hour}:{minute}:{second}</p>
    {content}
  </div>
</body>
</html>
```

## 文法
### 見出し
`# `, `## `, `### `, `#### `, `##### `, `###### `の後に見出しを書く。`#`の見出しは文書タイトルとなる。また見出しからは目次が自動で生成される。見出し中でも注やリンクを利用できるが、目次内では除外・無効化される。

### 強調
**Bold**(`**Bold**`)と__Italic__(`__Italic__`)を利用できる。

### リンク
[`[text](url)`でリンクを貼る。](#リンク)リンクテキストでリンクや注を使うことはできない。

リンクテキストを省略すると、URLのページの`<title>`要素からタイトルを抽出しリンクテキストとする。
[](https://season1618.github.io/notex/)

### 注[^横組の書物の場合、ページ下部に置かれるものを脚注(footnote)、本文が一区切りされる編・章・節の終わりに付けられるものを後注(endnote)と呼ぶ。]
`[^注]`と書くことで注を入れる。注の中で注は使えない[^注を更に補足する補注というものもあるが、一般的でないためネストはしないものとする。]。注は引用で参照へのリンクを貼る[^注は参照で引用へのリンクを貼る。]。`[^]`でそれより上にある注の引用の内まだ参照されていないものをリストする。`[^]`で回収されない注は文書の最後でまとめて回収される。

[^]

### リスト
- 順序なしリスト: `- `
    - 項目1
    - 項目2
    - 項目3
- 順序付きリスト: `+ `
    + 項目1
    + 項目2
    + 項目3

### 表
```
| 見出し1 | 見出し2 | 見出し3 |
-----------------
| aaa | bbb | ccc |
| aaa | bbb | ccc |
```

| 見出し1 | 見出し2 | 見出し3 |
-----------------
| aaa | bbb | ccc |
| aaa | bbb | ccc |

### 画像
@[`@[caption](url)`とすることで画像を挿入する](./image.jpg)

### リンクカード
OGP情報を取得しリンクカードを生成する。

```
@[](url)
```

@[](https://github.com/season1618/notex)

### 引用
>>
`>>`と`<<`で囲むことで引用となる。
<<

### 数式
インライン数式は`$ .. $`、数式ブロックは`$$ .. $$`。MathJaxを利用するためHTMLの`<head>`に
```html
<script src="https://polyfill.io/v3/polyfill.min.js?features=es6"></script>
<script id="MathJax-script" async src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"></script>
```
を記述。

状態を$u$、ハミルトニアンを$H$とすると、状態の時間発展は
$$
    u(t + dt) = \exp\left(-i\frac{H}{\hbar}dt\right)u(t)
$$
となる。

MathJaxのドキュメントは以下を参照。
@[](https://docs.mathjax.org/en/latest/)

### コード
インラインコードは`\` .. \``、コードブロックは`\`\`\` .. \`\`\``。

コードブロックにシンタックスハイライトを付けるにはHighlight.jsを利用する。HTMLの`<head>`に
```html
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css">
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/go.min.js"></script>
<script>hljs.highlightAll();</script>
```
を記述。

```c
#include <stdio.h>

int main() {
    printf("hello, world");
    return 0;
}
```

highlight.jsのドキュメントは以下を参照。
@[](https://highlightjs.readthedocs.io/en/latest/)

サポート言語とテーマ。
- [](https://github.com/highlightjs/highlight.js/blob/main/SUPPORTED_LANGUAGES.md)
- [](https://github.com/highlightjs/highlight.js/tree/main/src/styles)
- [](https://cdnjs.com/libraries/highlight.js)

## 形式文法
```
document = block*

block = header
      | quote
      | list
      | table
      | image
      | link-card
      | math-block
      | code-block
      | paragraph
      | ref
header = ("# " | "## " | "### " | "#### " | "##### " | "###### ") inline
quote = >> inline* <<
list = (("- " | "+ ") inline EOL)*
table = ("|" ( inline "|" )* EOL)* "-"+ EOL ("|" ( inline "|" )* EOL)*
image = @[ inline ]( url )
link-card = @[]( url )
math-block = $$ .. $$
code-block = \``` .. \```
paragraph = inline
ref = "[^]"

inline = cite*
cite = [^ link* ]
     | link
link = [ emph* ]( url )
     | emph
emph = ** emph* **
     | __ emph* __
     | prim
prim = math = $ .. $
     | code = ` .. `
     | text
```