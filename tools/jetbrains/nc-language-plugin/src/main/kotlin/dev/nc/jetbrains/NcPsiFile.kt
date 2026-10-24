package dev.nc.jetbrains

import com.intellij.extapi.psi.PsiFileBase
import com.intellij.psi.FileViewProvider

class NcPsiFile(viewProvider: FileViewProvider) : PsiFileBase(viewProvider, NcLanguage) {
    override fun getFileType() = viewProvider.fileType
    override fun toString(): String = "nC PSI file"
}
