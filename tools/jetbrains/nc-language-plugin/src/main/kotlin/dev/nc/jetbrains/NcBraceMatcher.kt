package dev.nc.jetbrains

import com.intellij.lang.BracePair
import com.intellij.lang.PairedBraceMatcher
import com.intellij.psi.PsiFile
import com.intellij.psi.tree.IElementType

class NcBraceMatcher : PairedBraceMatcher {
    override fun getPairs(): Array<BracePair> = arrayOf(
        BracePair(NcTokenTypes.LBRACE, NcTokenTypes.RBRACE, true),
        BracePair(NcTokenTypes.LPAREN, NcTokenTypes.RPAREN, false),
        BracePair(NcTokenTypes.LBRACKET, NcTokenTypes.RBRACKET, false)
    )

    override fun isPairedBracesAllowedBeforeType(
        lbraceType: IElementType,
        contextType: IElementType?
    ): Boolean = true

    override fun getCodeConstructStart(file: PsiFile?, openingBraceOffset: Int): Int = openingBraceOffset
}
