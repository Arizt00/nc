# Informar de un problema de seguridad

Las versiones de bootstrap son experimentales. La semántica manual/raw deja la gestión de memoria bajo responsabilidad del programa; no promete protección automática frente a accesos inválidos.

Si encuentras un fallo del compilador, del cargador o del proceso de distribución que pueda afectar a otras personas, prepara una reproducción mínima y conserva el hash del paquete. No publiques credenciales, información personal ni un payload contra un servicio ajeno.

Usa la opción privada Report a vulnerability en la pestaña Security cuando esté habilitada. Si esa opción no está disponible, abre una incidencia solicitando un canal privado sin incluir los detalles de explotación. No hay un plazo de respuesta garantizado durante el bootstrap.

Los paquetes publicados incluyen SHA-256 para detectar alteraciones respecto al archivo anunciado. Un hash de integridad no sustituye una firma criptográfica ni una auditoría del compilador.
