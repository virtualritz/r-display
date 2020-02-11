"""
Python binding to 3Delight's Nodal Scene Interface
"""

BAD_CONTEXT = 0

SCENE_ROOT = '.root'
SCENE_GLOBAL = '.global'
ALL_NODES = '.all'

import ctypes
import os
import platform

# Load 3Delight
if platform.system() == "Windows":
    _lib3delight = ctypes.cdll.LoadLibrary('3Delight')
elif platform.system() == "Darwin":
    __delight = os.getenv('DELIGHT')
    if __delight is None:
        __delight = '/Applications/3Delight'
    _lib3delight = ctypes.cdll.LoadLibrary(__delight + '/lib/lib3delight.dylib')
else:
    _lib3delight = ctypes.cdll.LoadLibrary('lib3delight.so')


class _NSIParam_t(ctypes.Structure):
    """
    Python version of the NSIParam_t struct to interface with the C API.
    """
    _fields_ = [
        ("name", ctypes.c_char_p),
        ("data", ctypes.c_void_p),
        ("type", ctypes.c_int),
        ("arraylength", ctypes.c_int),
        ("count", ctypes.c_size_t),
        ("flags", ctypes.c_int)
        ]

class Type:
    """
    Python version of the C NSIType_t enum
    """
    Invalid = 0
    Float = 1
    Double = Float | 0x10
    Integer = 2
    String = 3
    Color = 4
    Point = 5
    Vector = 6
    Normal = 7
    Matrix = 8
    DoubleMatrix = Matrix | 0x10
    Pointer = 9

_nsi_type_num_elements = {
    Type.Float : 1,
    Type.Double : 1,
    Type.Integer : 1,
    Type.String : 1,
    Type.Color : 3,
    Type.Point : 3,
    Type.Vector : 3,
    Type.Normal : 3,
    Type.Matrix : 16,
    Type.DoubleMatrix : 16,
    Type.Pointer : 1 }

class Flags:
    """
    Python version of the NSIParam_t flags values
    """
    IsArray = 1
    PerFace = 2
    PerVertex = 4
    InterpolateLinear = 8

def _GetArgNSIType(value):
    if isinstance(value, Arg):
        if value.type is not None:
            return value.type
        else:
            return _GetArgNSIType(value.value)
    if isinstance(value, (tuple, list)):
        return _GetArgNSIType(value[0])
    if isinstance(value, (int, bool)):
        return Type.Integer
    if isinstance(value, float):
        return Type.Double
    if isinstance(value, str):
        return Type.String
    return Type.Invalid

def _GetArgCType(value):
    nsitype = _GetArgNSIType(value)
    typemap = {
        Type.Float : ctypes.c_float,
        Type.Double : ctypes.c_double,
        Type.Integer : ctypes.c_int,
        Type.String : ctypes.c_char_p,
        Type.Color : ctypes.c_float,
        Type.Point : ctypes.c_float,
        Type.Vector : ctypes.c_float,
        Type.Normal : ctypes.c_float,
        Type.Matrix : ctypes.c_float,
        Type.DoubleMatrix : ctypes.c_double,
        Type.Pointer : ctypes.c_void_p
    }
    return typemap.get(nsitype)

def _BuildOneCArg(nsiparam, value):
    """
    Fill one _NSIParam_t object from an argument value.
    """
    # TODO: Support numpy.matrix as DoubleMatrix argument.
    datatype = _GetArgCType(value)
    arraylength = None
    countoverride = None
    flags = 0
    v = value
    if isinstance(value, Arg):
        v = value.value
        arraylength = value.arraylength
        countoverride = value.count
        flags = value.flags

    if v is None:
        nsiparam.type = Type.Invalid
        return

    if isinstance(v, ctypes.c_void_p):
        # Raw data given with ctypes. Must use nsi.Arg. No safety here.
        datacount = 0 # Will be overriden by countoverride.
        nsiparam.data = v
    elif isinstance(v, (tuple, list)):
        # Data is multiple values (eg. a list of floats).
        datacount = len(v)
        arraytype = datatype * datacount;
        if v and isinstance(v[0], str):
            # Encode all the strings to utf-8
            fixedv = [x.encode('utf-8') for x in v]
            nsiparam.data = ctypes.cast(ctypes.pointer(
                arraytype(*fixedv)), ctypes.c_void_p)
        else:
            nsiparam.data = ctypes.cast(ctypes.pointer(
                arraytype(*v)), ctypes.c_void_p)
    else:
        # Data is a single object (string, float, int).
        datacount = 1
        if isinstance(v, str):
            nsiparam.data = ctypes.cast(ctypes.pointer(
                datatype(v.encode('utf-8'))), ctypes.c_void_p)
        else:
            nsiparam.data = ctypes.cast(ctypes.pointer(
                datatype(v)), ctypes.c_void_p)

    valuecount = datacount

    nsitype = _GetArgNSIType(value)
    numelements = _nsi_type_num_elements.get(nsitype, 0)
    if numelements == 0:
        valuecount = 0
    else:
        valuecount = int(valuecount / numelements)

    if arraylength is not None:
        nsiparam.arraylength = arraylength
        flags |= Flags.IsArray
        if arraylength == 0:
            valuecount = 0
        else:
            valuecount = int(valuecount / arraylength)

    if countoverride is not None:
        if countoverride < valuecount or isinstance(v, ctypes.c_void_p):
            valuecount = countoverride

    nsiparam.type = nsitype
    nsiparam.count = valuecount
    nsiparam.flags = flags

def _BuildCArgumentList(args):
    cargs_type = _NSIParam_t * len(args)
    cargs = cargs_type()
    for i, arg in enumerate(args.items()):
        cargs[i].name = arg[0].encode('utf-8')
        _BuildOneCArg(cargs[i], arg[1])
    return cargs

class Context:
    """
    A NSI context.

    All NSI operations are done in a specific context. Multiple contexts may
    cohexist.

    Most methods of the Context accept a named argument list. The argument
    values can be native python integer, string or float (which is given to NSI
    as a double). More complex types should use one of the Arg classes in this
    module.
    """

    def __init__(self, handle=None):
        """
        If an integer handle argument is provided, this object will be bound to
        an existing NSI context with that handle.

        It is not required that the context have been created by the python
        binding.
        """
        self._handle = BAD_CONTEXT

    def Begin(self, **arglist):
        """
        Create a new NSI context and bind this object to it.
        """
        a = _BuildCArgumentList(arglist)
        self._handle = _lib3delight.NSIBegin(len(a), a)

    def End(self):
        """
        Release the context.

        If this object was bound to an external handle, that handle will on
        longer be valid after this call.
        """
        # TODO: Support with statement
        if self._handle != BAD_CONTEXT:
            _lib3delight.NSIEnd( self._handle )

    def Create(self, handle, type, **arglist):
        """
        Create a new node.

        Parameters
        handle : The handle of the node to create.
        type : The type of node to create.
        """
        a = _BuildCArgumentList(arglist)
        _lib3delight.NSICreate(
            self._handle,
            handle.encode('utf-8'),
            type.encode('utf-8'),
            len(a), a)

    def Delete(self, handle, **arglist):
        """
        Delete a node.

        Parameters
        handle : The handle of the node to delete.
        """
        a = _BuildCArgumentList(arglist)
        _lib3delight.NSIDelete(
            self._handle,
            handle.encode('utf-8'),
            len(a), a)

    def SetAttribute(self, handle, **arglist):
        """
        Set attributes of a node.

        Parameters
        handle : The handle of the node on which to set attributes.
        """
        a = _BuildCArgumentList(arglist)
        _lib3delight.NSISetAttribute(
            self._handle,
            handle.encode('utf-8'),
            len(a), a)

    def SetAttributeAtTime(self, handle, time, **arglist):
        """
        Set attributes of a node for a specific time.

        Parameters
        handle : The handle of the node on which to set attributes.
        time : The time for which the attributes are set.
        """
        a = _BuildCArgumentList(arglist)
        _lib3delight.NSISetAttributeAtTime(
            self._handle,
            handle.encode('utf-8'),
            ctypes.c_double(time),
            len(a), a)

    def DeleteAttribute(self, handle, name):
        """
        Delete an attribute of a node.

        Parameters
        handle : The handle of the node on which to delete an attribute.
        name : The name of the attribute to delete.
        """
        _lib3delight.NSIDeleteAttribute(
            self._handle,
            handle.encode('utf-8'),
            name.encode('utf-8'))

    def Connect(self, from_handle, from_attr, to_handle, to_attr, **arglist):
        """
        Connect nodes or specific attributes of nodes.

        Parameters
        from_handle : The handle of the node to connect from.
        from_attr : Optional attribute to connect from.
        to_handle : The handle of the node to connect to.
        to_attr : Optional attribute to connect to.
        """
        a = _BuildCArgumentList(arglist)
        _lib3delight.NSIConnect(
            self._handle,
            from_handle.encode('utf-8'),
            (from_attr if from_attr else '').encode('utf-8'),
            to_handle.encode('utf-8'),
            (to_attr if to_attr else '').encode('utf-8'),
            len(a), a)

    def Disconnect(self, from_handle, from_attr, to_handle, to_attr, **arglist):
        """
        Disconnect nodes or specific attributes of nodes.

        Parameters
        from_handle : The handle of the node to disconnect from.
        from_attr : Optional attribute to disconnect from.
        to_handle : The handle of the node to disconnect to.
        to_attr : Optional attribute to disconnect to.
        """
        a = _BuildCArgumentList(arglist)
        _lib3delight.NSIDisconnect(
            self._handle,
            from_handle.encode('utf-8'),
            (from_attr if from_attr else '').encode('utf-8'),
            to_handle.encode('utf-8'),
            (to_attr if to_attr else '').encode('utf-8'))

    def Evaluate(self, **arglist):
        """
        Evaluate NSI commands from some other source.

        This can read other files, run scripts, etc.
        """
        a = _BuildCArgumentList(arglist)
        _lib3delight.NSIEvaluate(
            self._handle,
            len(a), a)

    def RenderControl(self, **arglist):
        """
        Control rendering.

        This is used to start and stop renders, wait for them to complete, etc.
        """
        a = _BuildCArgumentList(arglist)
        _lib3delight.NSIRenderControl(
            self._handle,
            len(a), a)

class Arg:
    """
    Wrapper for NSI parameter list values.

    NSI functions which accept a parameter list may be given values wrapped in
    an Arg object to specify details about the argument. For example, 3 float
    values are normally output as 3 NSITypeDouble values. To set a color
    instead, give nsi.Arg((0.4, 0.2, 0.5), type=nsi.Type.Color)

    The most common types have specific wrappers which are easier to use. The
    above example could instead be nsi.ColorArg(0.4, 0.2, 0.5)
    """
    def __init__(self, v, type=None, arraylength=None, flags=None, count=None):
        """
        Parameters
        v : The value.
        type : An optional value from nsi.Type
        arraylength : An optional integer to specify the array length of the
        base type. For example, 2 for texture coordinates.
        flags : Optional flags from nsi.Flags
        count : Number of values of the base type.
        """
        self.type = None
        self.arraylength = None
        self.flags = 0
        self.count = None

        if isinstance(v, Arg):
            # Fold its attributes into this object. This allows chaining Arg
            # objects.
            self.value = v.value
            self.type = v.type
            self.flags = v.flags
            self.count = v.count
        else:
            self.value = v

        if arraylength is not None:
            self.arraylength = arraylength
        if type is not None:
            self.type = type
        if flags is not None:
            self.flags |= flags
        if count is not None:
            self.count = count

class IntegerArg(Arg):
    """
    Wrapper for NSI parameter list integer value.

    Use as nsi.IntegerArg(2). This is generally not needed as it is the default
    behavior when an int is given. Using this class will enforce the type.
    """
    def __init__(self, v):
        Arg.__init__(self, int(v), type=Type.Integer)

class FloatArg(Arg):
    """
    Wrapper for NSI parameter list float value.

    Use as nsi.FloatArg(0.5).
    """
    def __init__(self, v):
        Arg.__init__(self, v, type=Type.Float)

class DoubleArg(Arg):
    """
    Wrapper for NSI parameter list double value.

    Use as nsi.DoubleArg(0.5).
    """
    def __init__(self, v):
        Arg.__init__(self, v, type=Type.Double)

class ColorArg(Arg):
    """
    Wrapper for NSI parameter list color value.

    Use as nsi.ColorArg(0.2, 0.3, 0.4) or nsi.ColorArg(0.5)
    """
    def __init__(self, r, g=None, b=None):
        if b is None:
            Arg.__init__(self, (r,r,r), type=Type.Color)
        else:
            Arg.__init__(self, (r,g,b), type=Type.Color)

# vim: set softtabstop=4 expandtab shiftwidth=4:
