//-----------------------------------------------------------------------------
#ifndef PropDataH
#define PropDataH PropDataH
//-----------------------------------------------------------------------------
#include <mvIMPACT_CPP/mvIMPACT_acquire.h>
#include <wxPropGrid/Include/propgrid.h>
#include "PropTree.h"
#ifndef SWIG
#   include <string>
#endif

#ifndef wxPG_EXTRAS_DECL
#   ifdef __WXPYTHON__
// The Python wrapper is a dll
#       define wxPG_EXTRAS_DECL WXDLLIMPEXP_PG
#   else
// For a normal application
#       define wxPG_EXTRAS_DECL
#   endif
#endif

#ifndef SWIG
#   include <apps/Common/wxAbstraction.h>
#endif

WX_PG_DECLARE_STRING_PROPERTY_WITH_DECL( wxBinaryDataProperty, WXDLLIMPEXP_PG )

//------------------------------------------------------------------------------
class PropData
//------------------------------------------------------------------------------
{
public:
    WXDLLIMPEXP_PG enum ECtrl
    {
        _ctrlStatic,
        _ctrlSpinner,
        _ctrlEdit,
        _ctrlCombo,
        _ctrlMultiChoiceSelector,
        _ctrlFileSelector,
        _ctrlDirSelector,
        _ctrlBinaryDataEditor
    };
    WXDLLIMPEXP_PG explicit                     PropData( mvIMPACT::acquire::HOBJ hObj );
    WXDLLIMPEXP_PG virtual                     ~PropData( void ) {}
    WXDLLIMPEXP_PG static void                  AppendComponentInfo( const mvIMPACT::acquire::Component comp, wxString& info, unsigned int actChangedCount, unsigned int actAttrChangedCount );
    WXDLLIMPEXP_PG void                         AppendComponentInfo( wxString& info, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const;
    WXDLLIMPEXP_PG static void                  AppendSelectorInfo( std::ostringstream& oss, const std::vector<mvIMPACT::acquire::Component>& v );
    WXDLLIMPEXP_PG virtual void                 EnsureValidGridItem( const PropTree* pPropTree, wxPGId parentGridComponent, EDisplayFlags flags, bool* pboModified = 0  ) = 0;
    WXDLLIMPEXP_PG mvIMPACT::acquire::Component GetComponent( void ) const
    {
        return m_Component;
    }
    WXDLLIMPEXP_PG wxString                     GetFeatureFullName( void ) const
    {
        return m_FeatureFullName;
    }
    WXDLLIMPEXP_PG wxString                     GetDisplayName( EDisplayFlags flags ) const;
    WXDLLIMPEXP_PG wxPGId                       GetGridItem( void ) const
    {
        return m_GridItemId;
    }
    WXDLLIMPEXP_PG wxPropertyGrid*              GetParentGrid( void ) const
    {
        return m_pParentGrid;
    }
    WXDLLIMPEXP_PG ECtrl                        GetType( void ) const
    {
        return m_Type;
    }
    WXDLLIMPEXP_PG void                         InvalidatePropGridItem( void )
    {
        m_GridItemId = wxNullProperty;
    }
    WXDLLIMPEXP_PG bool                         HasChanged( void ) const
    {
        return m_Component.changedCounter() != m_lastChangedCounter;
    }
    WXDLLIMPEXP_PG virtual void                 OnExpand( void ) {}
    WXDLLIMPEXP_PG void                         SetFeatureFullName( const wxString& featureFullName )
    {
        m_FeatureFullName = featureFullName;
    }
    WXDLLIMPEXP_PG virtual void                 Update( const PropTree* pPropTree, EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const = 0;
    WXDLLIMPEXP_PG void                         UpdateGridItem( const PropTree* pPropTree, EDisplayFlags flags, bool* pboModified );
    WXDLLIMPEXP_PG virtual void                 UpdatePropData( void ) {}
protected:
    void                                        UpdateLabelAndHelpString( EDisplayFlags flags, wxString& label ) const;

    wxPGId                                      m_GridItemId;
    unsigned int                                m_lastChangedCounter;
    unsigned int                                m_lastChangedCounterAttr;
    wxPropertyGrid*                             m_pParentGrid;
    ECtrl                                       m_Type;
private:
    const wxColour&                             GetBackgroundColour( void ) const;
    bool                                        IsVisible( void ) const;

    const mvIMPACT::acquire::Component          m_Component;
    wxString                                    m_FeatureFullName;
};

//------------------------------------------------------------------------------
class MethodObject : public PropData
//------------------------------------------------------------------------------
{
public:
    WXDLLIMPEXP_PG explicit                     MethodObject( mvIMPACT::acquire::HOBJ hObj );
    WXDLLIMPEXP_PG static wxString              BuildFriendlyName( mvIMPACT::acquire::HOBJ hObj );
    WXDLLIMPEXP_PG wxString                     Call( int& callResult ) const;
    WXDLLIMPEXP_PG void                         EnsureValidGridItem( const PropTree* pPropTree, wxPGId parentItem, EDisplayFlags flags, bool* pboModified = 0  );
    WXDLLIMPEXP_PG const wxString&              FriendlyName( void ) const
    {
        return m_FriendlyName;
    }
    WXDLLIMPEXP_PG const wxString&              Params( void ) const
    {
        return m_Params;
    }
    WXDLLIMPEXP_PG void                         UpdatePropData( void );
    WXDLLIMPEXP_PG void                         Update( const PropTree* pPropTree, EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const;
private:
    WXDLLIMPEXP_PG wxString                     GetNameToUse( EDisplayFlags flags ) const;

    wxString                                    m_Params;
    wxString                                    m_FriendlyName;
};

//------------------------------------------------------------------------------
class ListObject : public PropData
//------------------------------------------------------------------------------
{
public:
    WXDLLIMPEXP_PG explicit                     ListObject( mvIMPACT::acquire::HOBJ hObj, const char* pTitle = 0 );
    WXDLLIMPEXP_PG bool                         IsExpanded( void ) const
    {
        return m_boExpanded;
    }
    WXDLLIMPEXP_PG void                         OnExpand( void );
    WXDLLIMPEXP_PG void                         EnsureValidGridItem( const PropTree* pPropTree, wxPGId parentItem, EDisplayFlags flags, bool* pboModified = 0  );
    WXDLLIMPEXP_PG void                         Update( const PropTree* pPropTree, EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const;
private:
    bool                                        m_boExpanded;
    const wxString                              m_Title;
};

//------------------------------------------------------------------------------
class PropertyObject : public PropData
//------------------------------------------------------------------------------
{
public:
    WXDLLIMPEXP_PG explicit                     PropertyObject( mvIMPACT::acquire::HOBJ hObj, int index = 0 );
    WXDLLIMPEXP_PG void                         UpdatePropData( void );
    WXDLLIMPEXP_PG void                         EnsureValidGridItem( const PropTree* pPropTree, wxPGId parentItem, EDisplayFlags flags, bool* pboModified = 0 );
    WXDLLIMPEXP_PG int                          GetIndex( void ) const
    {
        return m_Index;
    }
    WXDLLIMPEXP_PG void                         Update( const PropTree* pPropTree, EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const;
private:
    wxString                                    GetCurrentValueAsString( void ) const;
    void                                        GetTransformedDict( wxPGChoices& soc, wxString* pEmptyString = 0 ) const;
    bool                                        IsWriteable( void ) const;
    void                                        SetToLimit( const mvIMPACT::acquire::TPropertyLimits limit ) const;
    void                                        UpdateLabel( EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const;
    void                                        WritePropVal( const std::string& value ) const;
    void                                        WritePropVal( const std::string& value, const int index ) const;

    const int                                   m_Index;
    const bool                                  m_boVectorAsList;
};

//------------------------------------------------------------------------------
class VectorPropertyObject : public PropData
//------------------------------------------------------------------------------
{
public:
    WXDLLIMPEXP_PG explicit                     VectorPropertyObject( mvIMPACT::acquire::HOBJ hObj );
    WXDLLIMPEXP_PG                             ~VectorPropertyObject();
    WXDLLIMPEXP_PG bool                         IsExpanded( void ) const
    {
        return m_boExpanded;
    }
    WXDLLIMPEXP_PG void                         OnExpand( void );
    WXDLLIMPEXP_PG PropertyObject*              GetVectorItem( int index );
    WXDLLIMPEXP_PG void                         EnsureValidGridItem( const PropTree* pPropTree, wxPGId parentItem, EDisplayFlags flags, bool* pboModified = 0 );
    WXDLLIMPEXP_PG void                         RemoveValue( unsigned int index );
    WXDLLIMPEXP_PG void                         Resize( void );
    WXDLLIMPEXP_PG void                         Update( const PropTree* pPropTree, EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const;
private:
    void                                        DeleteGridProperty( size_t index );

    bool                                        m_boExpanded;
    std::vector<PropertyObject*>                m_VectorItems;
};

#endif // PropDataH
