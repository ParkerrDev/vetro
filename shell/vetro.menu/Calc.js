.pragma library

// Vetro: the calculator in the menu. A small parser of its own rather than
// eval(), since what's typed here is otherwise only ever matched against names.
//
//   2+2   3*(4.5-1)   2^10   15% of 80   sqrt(2)   sin(pi/2)   1e3/7   7 mod 3
//
// evaluate(text) returns the answer as text, or "" when the text isn't a sum
// (a plain number or a word isn't: "7" and "pi" still search for apps).

var FUNCTIONS = {
  sqrt: Math.sqrt, cbrt: Math.cbrt, abs: Math.abs, round: Math.round, floor: Math.floor, ceil: Math.ceil,
  sin: Math.sin, cos: Math.cos, tan: Math.tan, asin: Math.asin, acos: Math.acos, atan: Math.atan,
  ln: Math.log, log: Math.log10, log2: Math.log2, exp: Math.exp
}
var CONSTANTS = { pi: Math.PI, e: Math.E, tau: Math.PI * 2 }

function tokenize(text) {
  var tokens = []
  // 1,000,000 is a million; 1,5 is one and a half
  var s = text.toLowerCase().replace(/(\d),(?=\d{3}(?!\d))/g, "$1").replace(/×/g, "*").replace(/÷/g, "/").replace(/−/g, "-").replace(/\*\*/g, "^")
  var re = /(\d+(?:[.,]\d+)?(?:e[+-]?\d+)?|\.\d+)|([a-z]+)|([-+*\/^()%!,])|(\s+)|(.)/g
  var m
  while ((m = re.exec(s)) !== null) {
    if (m[1] !== undefined) tokens.push({ type: "num", value: parseFloat(m[1].replace(",", ".")) })
    else if (m[2] !== undefined) tokens.push({ type: "word", value: m[2] })
    else if (m[3] !== undefined) tokens.push({ type: "op", value: m[3] })
    else if (m[5] !== undefined) return null
  }
  return tokens
}

function Parser(tokens) {
  this.tokens = tokens
  this.pos = 0
  this.operations = 0
}
Parser.prototype.peek = function() { return this.tokens[this.pos] }
Parser.prototype.isOp = function(value) {
  var t = this.peek()
  return !!t && t.type === "op" && t.value === value
}
Parser.prototype.isWord = function(value) {
  var t = this.peek()
  return !!t && t.type === "word" && t.value === value
}
Parser.prototype.expression = function() {
  var left = this.term()
  while (this.isOp("+") || this.isOp("-")) {
    var op = this.tokens[this.pos++].value
    var right = this.term()
    left = op === "+" ? left + right : left - right
    this.operations++
  }
  return left
}
Parser.prototype.term = function() {
  var left = this.unary()
  for (;;) {
    if (this.isOp("*") || this.isOp("/")) {
      var op = this.tokens[this.pos++].value
      var right = this.unary()
      left = op === "*" ? left * right : left / right
    } else if (this.isWord("mod")) {
      this.pos++
      left = left % this.unary()
    } else if (this.isWord("x") && this.tokens[this.pos + 1] && this.tokens[this.pos + 1].type !== "op") {
      this.pos++
      left = left * this.unary()
    } else if (this.isWord("of")) {
      // "15% of 80"
      this.pos++
      left = left * this.unary()
    } else break
    this.operations++
  }
  return left
}
Parser.prototype.unary = function() {
  if (this.isOp("-")) { this.pos++; return -this.unary() }
  if (this.isOp("+")) { this.pos++; return this.unary() }
  return this.power()
}
Parser.prototype.power = function() {
  var base = this.postfix()
  if (this.isOp("^")) {
    this.pos++
    this.operations++
    return Math.pow(base, this.unary())
  }
  return base
}
Parser.prototype.postfix = function() {
  var value = this.primary()
  for (;;) {
    if (this.isOp("%")) { this.pos++; value = value / 100; this.operations++ }
    else if (this.isOp("!")) {
      this.pos++
      if (value < 0 || value > 170 || Math.floor(value) !== value) throw "factorial"
      var f = 1
      for (var i = 2; i <= value; i++) f *= i
      value = f
      this.operations++
    } else break
  }
  return value
}
Parser.prototype.primary = function() {
  var t = this.tokens[this.pos++]
  if (!t) throw "end"
  if (t.type === "num") return t.value
  if (t.type === "op" && t.value === "(") {
    var inner = this.expression()
    if (!this.isOp(")")) throw "paren"
    this.pos++
    return inner
  }
  if (t.type === "word") {
    if (FUNCTIONS[t.value] && this.isOp("(")) {
      this.pos++
      var arg = this.expression()
      if (!this.isOp(")")) throw "paren"
      this.pos++
      this.operations++
      return FUNCTIONS[t.value](arg)
    }
    if (CONSTANTS[t.value] !== undefined) return CONSTANTS[t.value]
  }
  throw "token"
}

function format(value) {
  if (!isFinite(value)) return ""
  if (value !== 0 && (Math.abs(value) >= 1e15 || Math.abs(value) < 1e-9)) return value.toExponential(8).replace(/\.?0+e/, "e")
  var rounded = parseFloat(value.toPrecision(12))
  var parts = String(rounded).split(".")
  parts[0] = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, ",")
  return parts.join(".")
}

function evaluate(text) {
  var input = String(text || "").replace(/^\s*=\s*/, "")
  if (!/\d/.test(input) && !/\b(pi|tau)\b/i.test(input)) return ""
  var tokens = tokenize(input)
  if (!tokens || tokens.length < 2) return ""
  try {
    var parser = new Parser(tokens)
    var value = parser.expression()
    if (parser.pos !== tokens.length || parser.operations === 0) return ""
    return format(value)
  } catch (e) {
    return ""
  }
}
