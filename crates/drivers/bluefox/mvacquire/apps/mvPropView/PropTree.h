//-----------------------------------------------------------------------------
#ifndef PropTreeH
#define PropTreeH PropTreeH
//-----------------------------------------------------------------------------
#include <mvIMPACT_CPP/mvIMPACT_acquire.h>
#include <map>
#include <string>
#include <wxPropGrid/Include/propgrid.h>

#ifndef wxPG_EXTRAS_DECL
#   ifdef __WXPYTHON__
// The Python wrapper is a dll
#       define wxPG_EXTRAS_DECL WXDLLIMPEXP_PG
#   else
#       define wxPG_EXTRAS_DECL
#   endif
#endif

class PropData;
#ifdef __WXPYTHON__
class MethodObject;
class ListObject;
class PropertyObject;
#endif
class VectorPropertyObject;

typedef std::map<mvIMPACT::acquire::HOBJ, PropData*> CompToDataMap;
typedef std::map<PropData*, mvIMPACT::acquire::HOBJ> DataToCompMap;

//------------------------------------------------------------------------------
enum WXDLLIMPEXP_PG EDisplayFlags
//------------------------------------------------------------------------------
{
    dfNone = 0x0,
    dfDisplayDebugInfo = 0x1,
    dfHexIndices = 0x2,
    dfDisplayInvisibleComponents = 0x4,
    dfDisplayNames = 0x8,
    dfSelectorGrouping = 0x10,
    dfDontUseFriendlyNamesForMethods = 0x20
};

//------------------------------------------------------------------------------
class PropTree
//------------------------------------------------------------------------------
{
    friend class VectorPropertyObject;
    friend class PropertyObject;
    mvIMPACT::acquire::ComponentIterator    m_rootList;
    wxPropertyGrid* const                   m_pPropGrid;
    wxPGId                                  m_TopLevelProp;
    const std::string                       m_Title;
    mutable CompToDataMap                   m_CompToDataMap;
    mutable DataToCompMap                   m_DataToCompMap;
    const EDisplayFlags                     m_flags;

    wxString                                BuildFullFeatureName( Component comp ) const;
    wxPGId                                  CreateGridProperty( mvIMPACT::acquire::HOBJ hObj, wxPGId parentProp, int index = -1, bool* pboModified = 0, const char* pTitle = 0 ) const;
    void                                    Delete( void );
    void                                    RemoveFromGlobalNameToFeatureMap( PropData* pPropData ) const;
    void                                    UpdateGridPropsRecursively( mvIMPACT::acquire::ComponentIterator iter, wxPGId prop, bool boForceRedraw ) const;
    void                                    UpdateGridPropsRecursively2( wxPGId listRoot, bool boForceDelete = false ) const;
public:
    WXDLLIMPEXP_PG                          PropTree( mvIMPACT::acquire::HOBJ hObj, const char* pTitle, wxPropertyGrid* pPropGrid, EDisplayFlags flags );
    WXDLLIMPEXP_PG                         ~PropTree( void );
    WXDLLIMPEXP_PG unsigned int             Draw( bool boForceRedraw );
#ifndef SWIG
    WXDLLIMPEXP_PG PropData*                GetPropData( mvIMPACT::acquire::HOBJ hObj ) const;
#endif
#ifdef __WXPYTHON__
    WXDLLIMPEXP_PG MethodObject*            GetMethodObject( mvIMPACT::acquire::HOBJ hObj ) const
    {
        return reinterpret_cast<MethodObject*>( GetPropData( hObj ) );
    }
    WXDLLIMPEXP_PG ListObject*              GetListObject( mvIMPACT::acquire::HOBJ hObj ) const
    {
        return reinterpret_cast<ListObject*>( GetPropData( hObj ) );
    }
    WXDLLIMPEXP_PG PropertyObject*          GetPropertyObject( mvIMPACT::acquire::HOBJ hObj ) const
    {
        return reinterpret_cast<PropertyObject*>( GetPropData( hObj ) );
    }
    WXDLLIMPEXP_PG VectorPropertyObject*    GetVectorPropertyObject( mvIMPACT::acquire::HOBJ hObj ) const
    {
        return reinterpret_cast<VectorPropertyObject*>( GetPropData( hObj ) );
    }
#endif
    WXDLLIMPEXP_PG unsigned int             GetChangedCounter( void ) const
    {
        return m_rootList.isValid() ? m_rootList.changedCounter() : 0;
    };
    WXDLLIMPEXP_PG const char*              GetTitle( void ) const
    {
        return m_Title.empty() ? "" : m_Title.c_str();
    }
    WXDLLIMPEXP_PG wxPropertyGrid*          GetPropGrid( void ) const
    {
        return m_pPropGrid;
    }
};

#endif // PropTreeH
