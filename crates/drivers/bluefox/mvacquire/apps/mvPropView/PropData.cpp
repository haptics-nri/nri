//-----------------------------------------------------------------------------
#include "DataConversion.h"
#include <limits>
#include "PropTree.h"
#include "PropData.h"
#include "SpinEditorDouble.h"
#include "ValuesFromUserDlg.h"
#include <vector>
#include <wxPropGrid/Include/propgrid.h>
#include <wxPropGrid/Include/advprops.h>
#include <wx/msgdlg.h>
#include <wx/settings.h>
#include <wx/stopwatch.h>

using namespace std;
using namespace mvIMPACT::acquire;

//-----------------------------------------------------------------------------
class wxBinaryDataPropertyClass : public wxLongStringPropertyClass
//-----------------------------------------------------------------------------
{
    WX_PG_DECLARE_DERIVED_PROPERTY_CLASS()
public:
    wxBinaryDataPropertyClass( const wxString& name, const wxString& label, const wxString& value ) : wxLongStringPropertyClass( name, label, value ) {}
    virtual ~wxBinaryDataPropertyClass() {}
    WX_PG_DECLARE_VALIDATOR_METHODS()
    static wxValidator* GetClassValidator();
    virtual bool OnButtonClick( wxPropertyGrid* pPropGrid, wxString& value )
    {
        BinaryDataDlg dlg( pPropGrid, GetLabel(), value );
        if( dlg.ShowModal() == wxID_OK )
        {
            value = dlg.GetBinaryData();
            return true;
        }
        return false;
    }
};

WX_PG_IMPLEMENT_DERIVED_PROPERTY_CLASS( wxBinaryDataProperty, wxLongStringProperty, const wxString& )

#if wxUSE_VALIDATORS
//-----------------------------------------------------------------------------
wxValidator* wxBinaryDataPropertyClass::GetClassValidator( void )
//-----------------------------------------------------------------------------
{
    WX_PG_DOGETVALIDATOR_ENTRY()
    // Atleast wxPython 2.6.2.1 required that the string argument is given
    static wxString v;
    HEXStringValidator* validator = new HEXStringValidator( &v );
    WX_PG_DOGETVALIDATOR_EXIT( validator )
}

//-----------------------------------------------------------------------------
wxValidator* wxBinaryDataPropertyClass::DoGetValidator( void ) const
//-----------------------------------------------------------------------------
{
    return GetClassValidator();
}
#endif

//=============================================================================
//========================= PropData ==========================================
//=============================================================================
//-----------------------------------------------------------------------------
PropData::PropData( HOBJ hObj ) : m_GridItemId( wxNullProperty ), m_lastChangedCounter( numeric_limits<unsigned int>::max() ),
    m_lastChangedCounterAttr( numeric_limits<unsigned int>::max() ), m_pParentGrid( 0 ), m_Component( hObj ), m_FeatureFullName() {}
//-----------------------------------------------------------------------------

//------------------------------------------------------------------------------
void PropData::AppendComponentInfo( mvIMPACT::acquire::Component comp, wxString& info, unsigned int actChangedCount, unsigned int actAttrChangedCount )
//------------------------------------------------------------------------------
{
    ostringstream oss;
    vector<mvIMPACT::acquire::Component> selectingFeatures;
    comp.selectingFeatures( selectingFeatures );
    vector<mvIMPACT::acquire::Component> selectedFeatures;
    comp.selectedFeatures( selectedFeatures );
    if( !selectingFeatures.empty() || !selectedFeatures.empty() )
    {
        AppendSelectorInfo( oss, selectingFeatures );
        AppendSelectorInfo( oss, selectedFeatures );
    }
    oss << "[" << comp.visibilityAsString()[0] << "]";
    info.Append( ConvertedString( oss.str() ) );

    info.Append( wxString::Format( wxT( ", hObj: 0x%08x, cc(%d/%d), type: %s, flags: %s" ),
                                   comp.hObj(), actChangedCount, actAttrChangedCount,
                                   ConvertedString( comp.typeAsString() ).c_str(),
                                   ConvertedString( comp.flagsAsString() ).c_str() ) );

    if( comp.isProp() )
    {
        const Property prop( comp );
        if( prop.hasMinValue() )
        {
            info.Append( wxString::Format( wxT( ", min: %s" ), ConvertedString( prop.readS( plMinValue ) ).c_str() ) );
        }
        if( prop.hasMaxValue() )
        {
            info.Append( wxString::Format( wxT( ", max: %s" ), ConvertedString( prop.readS( plMaxValue ) ).c_str() ) );
        }
        if( prop.hasStepWidth() )
        {
            info.Append( wxString::Format( wxT( ", inc: %s" ), ConvertedString( prop.readS( plStepWidth ) ).c_str() ) );
        }
    }
}

//------------------------------------------------------------------------------
void PropData::AppendComponentInfo( wxString& info, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const
//------------------------------------------------------------------------------
{
    AppendComponentInfo( m_Component, info, actChangedCount, actAttrChangedCount );
}

//-----------------------------------------------------------------------------
void PropData::AppendSelectorInfo( std::ostringstream& oss, const std::vector<mvIMPACT::acquire::Component>& v )
//-----------------------------------------------------------------------------
{
    const vector<mvIMPACT::acquire::Component>::size_type cnt = v.size();
    oss << "[";
    for( vector<mvIMPACT::acquire::Component>::size_type i = 0; i < cnt; i++ )
    {
        if( i > 0 )
        {
            oss << ", ";
        }
        oss << v[i].name();
    }
    oss << "]";
}

//-----------------------------------------------------------------------------
const wxColour& PropData::GetBackgroundColour( void ) const
//-----------------------------------------------------------------------------
{
    if( !m_Component.isVisible() )
    {
        return GlobalDataStorage::Instance()->GetPropGridColour( GlobalDataStorage::pgcInvisibleFeature );
    }

    if( !GlobalDataStorage::Instance()->IsComponentVisibilitySupported() ||
        ( m_Component.visibility() <= GlobalDataStorage::Instance()->GetComponentVisibility() ) )
    {
        return ( m_Component.isList() ? GlobalDataStorage::Instance()->GetPropGridColour( GlobalDataStorage::pgcListBackground ) : *wxWHITE );
    }

    switch( m_Component.visibility() )
    {
    case cvExpert:
        return GlobalDataStorage::Instance()->GetPropGridColour( GlobalDataStorage::pgcInvisibleExpertFeature );
    case cvGuru:
        return GlobalDataStorage::Instance()->GetPropGridColour( GlobalDataStorage::pgcInvisibleGuruFeature );
    case cvInvisible:
    default:
        break;
    }

    return GlobalDataStorage::Instance()->GetPropGridColour( GlobalDataStorage::pgcInvisibleFeature );
}

//-----------------------------------------------------------------------------
wxString PropData::GetDisplayName( EDisplayFlags flags ) const
//-----------------------------------------------------------------------------
{
    if( flags & dfDisplayNames )
    {
        const string displayName( m_Component.displayName() );
        return ConvertedString( displayName.empty() ? m_Component.name() : displayName );
    }
    return ConvertedString( m_Component.name() );
}

//-----------------------------------------------------------------------------
bool PropData::IsVisible( void ) const
//-----------------------------------------------------------------------------
{
    return ( m_Component.isVisible() &&
             ( !GlobalDataStorage::Instance()->IsComponentVisibilitySupported() || ( m_Component.visibility() <= GlobalDataStorage::Instance()->GetComponentVisibility() ) ) );
}

//------------------------------------------------------------------------------
void PropData::UpdateGridItem( const PropTree* pPropTree, EDisplayFlags flags, bool* pboModified )
//------------------------------------------------------------------------------
{
    const unsigned int actChangedCount = m_Component.changedCounter();
    const unsigned int actAttrChangedCount = m_Component.changedCounterAttr();

    if( ( m_lastChangedCounter != actChangedCount ) || ( m_lastChangedCounterAttr != actAttrChangedCount ) )
    {
        Update( pPropTree, flags, actChangedCount, actAttrChangedCount );
        m_lastChangedCounter = m_Component.changedCounter();
        m_lastChangedCounterAttr = m_Component.changedCounterAttr();
        if( pboModified )
        {
            *pboModified = true;
        }
    }
    else if( pboModified )
    {
        *pboModified = false;
    }

    if( flags & dfDisplayInvisibleComponents )
    {
#if wxPROPGRID_MINOR > 2
        m_pParentGrid->SetPropertyBackgroundColour( m_GridItemId, GetBackgroundColour() );
#else
        m_pParentGrid->SetPropertyPriority( m_GridItemId, wxPG_HIGH );
        m_pParentGrid->SetPropertyColour( m_GridItemId, GetBackgroundColour() );
    }
    else
    {
        m_pParentGrid->SetPropertyPriority( m_GridItemId, IsVisible() ? wxPG_HIGH : wxPG_LOW );
#endif
    }
}

//-----------------------------------------------------------------------------
void PropData::UpdateLabelAndHelpString( EDisplayFlags flags, wxString& label ) const
//-----------------------------------------------------------------------------
{
    const string docString( GetComponent().docString() );
    if( ( flags & dfDisplayDebugInfo ) && docString.empty() )
    {
        label.Prepend( wxT( "?" ) );
    }
    wxPGIdToPtr( m_GridItemId )->SetLabel( label );
    wxPGIdToPtr( m_GridItemId )->SetHelpString( ConvertedString( docString.empty() ? GetComponent().typeAsString() : docString ) );
}

#if wxPROPGRID_MINOR > 2
#   define NOW_NEW new
#else
#   define NOW_NEW
#endif

//=============================================================================
//========================= MethodObject ======================================
//=============================================================================
//-----------------------------------------------------------------------------
MethodObject::MethodObject( HOBJ hObj ) : PropData( hObj ), m_Params( wxT( "" ) ),
    m_FriendlyName( BuildFriendlyName( hObj ) )
//-----------------------------------------------------------------------------
{

}

//-----------------------------------------------------------------------------
wxString MethodObject::BuildFriendlyName( HOBJ hObj )
//-----------------------------------------------------------------------------
{
    wxString friendlyName;
    try
    {
        Method m( hObj );
        string::size_type end = 0;
        string name( m.name() );
        if( ( end = name.find_first_of( "@" ) ) != string::npos )
        {
            name = name.substr( 0, end );
        }
        const string para_type_str = m.paramList();
        string::const_iterator it = para_type_str.begin();
        friendlyName = ConvertedString( charToType( *it++ ) );
        friendlyName += wxT( " " );
        friendlyName += ConvertedString( name );
        friendlyName += wxT( "( " );
        const string::const_iterator itEND = para_type_str.end();
        while( it != itEND )
        {
            friendlyName += ConvertedString( charToType( *it ) );
            ++it;
            if( it != itEND )
            {
                friendlyName += wxT( ", " );
            }
        }
        friendlyName += wxT( " )" );
    }
    catch( const ImpactAcquireException& ) {}
    return friendlyName;
}

//------------------------------------------------------------------------------
wxString MethodObject::Call( int& callResult ) const
//------------------------------------------------------------------------------
{
    if( wxPGIdIsOk( m_GridItemId ) && GetComponent().isMeth() )
    {
        const wxString params = wxPGIdToPtr( m_GridItemId )->GetValueAsString();
        const Method meth( GetComponent() );
        bool boErrorHandled = false;
        long executionTime_ms = 0;
        const string paramsANSI( params.mb_str() );
        wxStopWatch stopWatch;
        try
        {
            wxBusyCursor busyCursorScope;
            callResult = meth.call( paramsANSI, " " );
            executionTime_ms = stopWatch.Time();
        }
        catch( const ImpactAcquireException& e )
        {
            executionTime_ms = stopWatch.Time();
            callResult = e.getErrorCode();
            wxString errorString( wxString::Format( wxT( "An error occurred while executing function '%s'(actual driver feature name: '%s'). %s(numerical error representation: %d (%s))." ), m_FriendlyName.c_str(), ConvertedString( GetComponent().name() ).c_str(), ConvertedString( e.getErrorString() ).c_str(), e.getErrorCode(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
            if( e.getErrorCode() == PROPHANDLING_WRONG_PARAM_COUNT )
            {
                errorString.Append( wxString::Format( wxT( "\n\nWhen executing methods please make sure you have specified the correct amount of parameters.\nThe following parameter types are available:\n  'void': This function either doesn't expect parameters or does not return a value\n  'void*': An arbitratry pointer\n  'int': A 32-bit integer value\n  'int64': A 64-bit integer value\n  'float': A double precision floating type value\n  'char*': A C-type string\nTherefore a function 'int foobar(float, char*) will expect one floating point value and one C-type string(in this order) and will return an integer value. Before executing a method the desired parameters must be confirmed by pressing [ENTER] in the edit box.\n\n" ) ) );
            }
            errorString.append( wxString::Format( wxT( "Error origin: %s" ), ConvertedString( e.getErrorOrigin() ).c_str() ) );
            wxMessageDialog errorDlg( NULL, errorString, wxT( "Method Execution Failed" ), wxOK | wxICON_INFORMATION );
            errorDlg.ShowModal();
            boErrorHandled = true;
        }

        wxString resultMsg( wxString::Format( wxT( "[ Last call info: %s( %s ), execution time: %ld ms" ), m_FriendlyName.c_str(), params.c_str(), executionTime_ms ) );
        const char retType = meth.paramList()[0];
        if( ( retType == 'v' ) || boErrorHandled )
        {
            callResult = DMR_NO_ERROR;
        }

        if( retType != 'v' )
        {
            resultMsg.append( wxString::Format( wxT( " = %s(%d)" ), ConvertedString( ImpactAcquireException::getErrorCodeAsString( callResult ) ).c_str(), callResult ) );
        }
        resultMsg.append( wxT( " ]" ) );
        return resultMsg;
    }
    return wxString( wxT( "Invalid method object" ) );
}

//------------------------------------------------------------------------------
wxString MethodObject::GetNameToUse( EDisplayFlags flags ) const
//------------------------------------------------------------------------------
{
    return ( flags & dfDontUseFriendlyNamesForMethods ) ? ConvertedString( GetComponent().name() ) : m_FriendlyName;
}

//------------------------------------------------------------------------------
void MethodObject::UpdatePropData( void )
//------------------------------------------------------------------------------
{
    if( wxPGIdIsOk( m_GridItemId ) )
    {
        m_Params = wxPGIdToPtr( m_GridItemId )->GetValueAsString();
    }
}

//------------------------------------------------------------------------------
void MethodObject::Update( const PropTree* /*pPropTree*/, EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const
//------------------------------------------------------------------------------
{
    wxString label( GetNameToUse( flags ) );
    if( flags & dfDisplayDebugInfo )
    {
        AppendComponentInfo( label, actChangedCount, actAttrChangedCount );
    }
    if( wxPGIdIsOk( m_GridItemId ) )
    {
        wxPGIdToPtr( m_GridItemId )->SetValueFromString( m_Params );
        UpdateLabelAndHelpString( flags, label );
    }
}

//------------------------------------------------------------------------------
void MethodObject::EnsureValidGridItem( const PropTree* pPropTree, wxPGId parentItem, EDisplayFlags flags, bool* pboModified /* = 0 */ )
//------------------------------------------------------------------------------
{
    if( !wxPGIdIsOk( m_GridItemId ) )
    {
        m_pParentGrid = pPropTree->GetPropGrid();
        m_GridItemId = m_pParentGrid->AppendIn( parentItem, NOW_NEW wxStringProperty( GetNameToUse( flags ), wxPG_LABEL, wxT( "" ) ) );
        m_pParentGrid->SetPropertyEditor( m_GridItemId, wxPG_EDITOR( TextCtrlAndButton ) );
        m_pParentGrid->SetPropertyClientData( m_GridItemId, this );
        m_Type = _ctrlEdit;
        if( pboModified )
        {
            *pboModified = true;
        }
    }
}

//=============================================================================
//========================= ListObject ========================================
//=============================================================================
//-----------------------------------------------------------------------------
ListObject::ListObject( HOBJ hObj, const char* pTitle /* = 0 */ )
    : PropData( hObj ), m_boExpanded( FALSE ), m_Title( ConvertedString( pTitle ? pTitle : string() ) )
//-----------------------------------------------------------------------------
{
}

//-----------------------------------------------------------------------------
void ListObject::OnExpand()
//------------------------------------------------------------------------------
{
    if( wxPGIdIsOk( m_GridItemId ) && m_pParentGrid )
    {
        m_boExpanded = m_pParentGrid->IsPropertyExpanded( m_GridItemId );
    }

    //UpdateLabel();
}

//------------------------------------------------------------------------------
void ListObject::EnsureValidGridItem( const PropTree* pPropTree, wxPGId parentItem, EDisplayFlags flags, bool* pboModified/* = 0 */ )
//------------------------------------------------------------------------------
{
    if( !wxPGIdIsOk( m_GridItemId ) )
    {
        m_pParentGrid = pPropTree->GetPropGrid();
#if wxPROPGRID_MINOR > 2
        // wxPGPropertyWithChildren is now abstract
        wxPropertyCategory* const prop = new wxPropertyCategory( GetDisplayName( flags ) );
#else
        wxPGPropertyWithChildren* const prop = new wxParentPropertyClass( GetDisplayName( flags ), wxPG_LABEL );
#endif
        // strictly speaking, prop could be NULL
        m_GridItemId = m_pParentGrid->AppendIn( parentItem, prop );
        if( m_GridItemId != prop )
        {
            wxASSERT( !"Invalid parenting" ); // no need to delete prop, wxPropertyGridState::PrepareToAddItem already did that
        }
        m_Type = _ctrlStatic;
        m_pParentGrid->SetPropertyClientData( m_GridItemId, this );
        const ComponentList list( GetComponent() );
        ConvertedString contDesc( list.contentDescriptor() );
        if( !contDesc.empty() )
        {
            wxPGIdToPtr( m_GridItemId )->SetValueFromString( contDesc );
            m_pParentGrid->SetPropertyTextColour( m_GridItemId, *wxBLUE );
        }
#if wxPROPGRID_MINOR > 2
        m_pParentGrid->SetPropertyBackgroundColour( m_GridItemId, GlobalDataStorage::Instance()->GetPropGridColour( GlobalDataStorage::pgcListBackground ) );
#else
        m_pParentGrid->SetPropertyColour( m_GridItemId, GlobalDataStorage::Instance()->GetPropGridColour( GlobalDataStorage::pgcListBackground ) );
#endif
        m_pParentGrid->DisableProperty( m_GridItemId );
        if( pboModified )
        {
            *pboModified = true;
        }
    }
}

//------------------------------------------------------------------------------
void ListObject::Update( const PropTree* /*pPropTree*/, EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const
//------------------------------------------------------------------------------
{
    if( ( ( flags & dfDisplayDebugInfo ) && ( actChangedCount != m_lastChangedCounter ) ) ||
        ( actAttrChangedCount != m_lastChangedCounterAttr ) )
    {
        wxString label;
        if( m_Title.empty() )
        {
            label = GetDisplayName( flags );
        }
        else
        {
            label = m_Title;
        }
        if( flags & dfDisplayDebugInfo )
        {
            label.Append( wxString::Format( wxT( "[%d]" ), ComponentList( GetComponent() ).size() ) );
            AppendComponentInfo( label, actChangedCount, actAttrChangedCount );
        }
        UpdateLabelAndHelpString( flags, label );
        const ComponentList list( GetComponent() );
        ConvertedString contDesc( list.contentDescriptor() );
        if( !contDesc.empty() )
        {
            wxPGIdToPtr( m_GridItemId )->SetValueFromString( contDesc );
            m_pParentGrid->SetPropertyTextColour( m_GridItemId, *wxBLUE );
        }
    }
}

//=============================================================================
//========================= PropertyObject ====================================
//=============================================================================
//-----------------------------------------------------------------------------
PropertyObject::PropertyObject( HOBJ hObj, int index /* = 0 */ )
    : PropData( hObj ), m_Index( index < 0 ? 0 : index ), m_boVectorAsList( index >= 0 )
//-----------------------------------------------------------------------------
{

}

//-----------------------------------------------------------------------------
wxString PropertyObject::GetCurrentValueAsString( void ) const
//-----------------------------------------------------------------------------
{
    if( ( GetComponent().type() == ctPropString ) && ( GetComponent().flags() & cfContainsBinaryData ) )
    {
        // The 'GetComponent().type()' check is only needed because some drivers with versions < 1.12.33
        // did incorrectly specify the 'cfContainsBinaryData' flag even though the data type was not 'ctPropString'...
        return ConvertedString( BinaryDataToString( PropertyS( GetComponent() ).readBinary( m_Index ) ) );
    }
    else
    {
        return ConvertedString( Property( GetComponent() ).readS( m_Index, string( ( m_Type == _ctrlMultiChoiceSelector ) ? "\"%s\" " : "" ) ) );
    }
}

//-----------------------------------------------------------------------------
bool PropertyObject::IsWriteable( void ) const
//-----------------------------------------------------------------------------
{
    bool boForceReadOnly = false;

    switch( GetType() )
    {
    case _ctrlMultiChoiceSelector:
        m_pParentGrid->SetPropertyValueString( m_GridItemId, GetCurrentValueAsString() );
        if( GetComponent().isProp() )
        {
            Property prop( GetComponent() );
            if( prop.dictSize() <= 1 )
            {
                boForceReadOnly = true;
            }
        }
        break;
    default:
        break;
    }

    return GetComponent().isWriteable() && !boForceReadOnly;
}

//-----------------------------------------------------------------------------
void PropertyObject::WritePropVal( const string& value ) const
//-----------------------------------------------------------------------------
{
    if( !IsWriteable() )
    {
        wxASSERT( !"Trying to write to a non-writeable property" );
    }

    if( m_boVectorAsList )
    {
        // only write one value to the property!
        const string::size_type start = value.find_first_not_of( " " );
        if( start == string::npos )
        {
            wxMessageDialog errorDlg( NULL, wxT( "Couldn't set value(Empty string?)" ), wxT( "Error" ), wxOK | wxICON_INFORMATION );
            errorDlg.ShowModal();
        }
        else
        {
            const string::size_type end = value.find_first_of( " ", start );
            WritePropVal( ( end != string::npos ) ? value.substr( start, end ) : value, m_Index );
        }
    }
    else
    {
        WritePropVal( value, 0 );
    }
}

//-----------------------------------------------------------------------------
void PropertyObject::WritePropVal( const string& value, const int index ) const
//-----------------------------------------------------------------------------
{
    if( ( GetComponent().type() == ctPropString ) && ( GetComponent().flags() & cfContainsBinaryData ) )
    {
        // The 'GetComponent().type()' check is only needed because some drivers with versions < 1.12.33
        // did incorrectly specify the 'cfContainsBinaryData' flag even though the data type was not 'ctPropString'...
        const PropertyS prop( GetComponent() );
        prop.writeBinary( BinaryDataFromString( value ), index );
    }
    else
    {
        const Property prop( GetComponent() );
        prop.writeS( value, index );
    }
}

//-----------------------------------------------------------------------------
void PropertyObject::SetToLimit( const mvIMPACT::acquire::TPropertyLimits limit ) const
//-----------------------------------------------------------------------------
{
    if( !IsWriteable() )
    {
        wxASSERT( !"Trying to write to a non-writeable property" );
    }

    switch( GetComponent().type() )
    {
    case ctPropInt:
        {
            PropertyI prop( GetComponent() );
            prop.write( prop.read( limit ) );
        }
        break;
    case ctPropInt64:
        {
            PropertyI64 prop( GetComponent() );
            prop.write( prop.read( limit ) );
        }
        break;
    case ctPropFloat:
        {
            PropertyF prop( GetComponent() );
            prop.write( prop.read( limit ) );
        }
        break;
    default:
        {
            Property prop( GetComponent() );
            prop.writeS( prop.readS( limit ) );
        }
        break;
    }
}

//------------------------------------------------------------------------------
void PropertyObject::UpdatePropData( void )
//------------------------------------------------------------------------------
{
    if( wxPGIdIsOk( m_GridItemId ) )
    {
        wxString errorString;
        const Property prop( GetComponent() );
        try
        {
            string valueANSI( wxPGIdToPtr( m_GridItemId )->GetValueAsString().mb_str() );
            WritePropVal( valueANSI );
        }
        catch( const EValTooSmall& )
        {
            errorString = wxT( "Value too small! Clipping to minimum!" );
            try
            {
                SetToLimit( plMinValue );
            }
            catch( const ImpactAcquireException& e )
            {
                errorString.Append( wxString::Format( wxT( " Can't set value( Error: %s(%s))!" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
            }
        }
        catch( const EValTooLarge& )
        {
            errorString = wxT( "Value too large! Clipping to maximum!" );
            try
            {
                SetToLimit( plMaxValue );
            }
            catch( const ImpactAcquireException& e )
            {
                errorString.Append( wxString::Format( wxT( " Can't set value( Error: %s(%s))!" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() ) );
            }
        }
        catch( const ENoModifySizeRights& )
        {
            errorString = wxString::Format( wxT( "This property can store %d value(s) only!" ), prop.valCount() );
        }
        catch( const EValidationFailed& )
        {
            errorString = wxT( "Value didn't pass validation check. The device log-file will contain additional information!" );
        }
        catch( const ImpactAcquireException& e )
        {
            errorString = wxString::Format( wxT( "Can't set value( Error: %s(%s))!" ), ConvertedString( e.getErrorString() ).c_str(), ConvertedString( e.getErrorCodeAsString() ).c_str() );
        }

        wxPGIdToPtr( m_GridItemId )->SetValueFromString( GetCurrentValueAsString() );
        if( !errorString.IsEmpty() )
        {
            wxMessageDialog errorDlg( NULL, errorString, wxT( "Error" ), wxOK | wxICON_INFORMATION );
            errorDlg.ShowModal();
        }
    }
}

//-----------------------------------------------------------------------------
void PropertyObject::GetTransformedDict( wxPGChoices& soc, wxString* pEmptyString /* = 0 */ ) const
//-----------------------------------------------------------------------------
{
    const TComponentType type = GetComponent().type();
    if( type == ctPropInt )
    {
        const TComponentFlag flags = GetComponent().flags();
        vector<pair<string, int> > dict;
        PropertyI( GetComponent() ).getTranslationDict( dict );
        vector<pair<string, int> >::size_type vSize = dict.size();
        for( vector<pair<string, int> >::size_type i = 0; i < vSize; i++ )
        {
            if( ( flags & cfAllowValueCombinations ) && ( dict[i].second == 0 ) )
            {
                if( pEmptyString )
                {
                    *pEmptyString = ConvertedString( dict[i].first );
                }
            }
            else
            {
                soc.Add( ConvertedString( dict[i].first ), dict[i].second );
            }
        }
    }
    else if( type == ctPropInt64 )
    {
        // this just works as long as we always use the string values for setting properties as there are no controls
        // for 64 bit integer values so far
        const TComponentFlag flags = GetComponent().flags();
        vector<pair<string, int64_type> > dict;
        PropertyI64( GetComponent() ).getTranslationDict( dict );
        vector<pair<string, int64_type> >::size_type vSize = dict.size();
        for( vector<pair<string, int64_type> >::size_type i = 0; i < vSize; i++ )
        {
            if( ( flags & cfAllowValueCombinations ) && ( dict[i].second == 0 ) )
            {
                if( pEmptyString )
                {
                    *pEmptyString = ConvertedString( dict[i].first );
                }
            }
            else
            {
                soc.Add( ConvertedString( dict[i].first ), dict[i].second );
            }
        }
    }
    else if( type == ctPropFloat )
    {
        // this just works as long as we always use the string values for setting properties as there are no controls
        // for double values so far
        vector<pair<string, double> > dict;
        PropertyF( GetComponent() ).getTranslationDict( dict );
        vector<pair<string, double> >::size_type vSize = dict.size();
        for( vector<pair<string, double> >::size_type i = 0; i < vSize; i++ )
        {
            soc.Add( ConvertedString( dict[i].first ), static_cast<int>( i ) );
        }
    }
    else
    {
        wxASSERT( !"Invalid component type for a combo box control" );
    }
}

//------------------------------------------------------------------------------
void PropertyObject::EnsureValidGridItem( const PropTree* pPropTree, wxPGId parentItem, EDisplayFlags flags, bool* pboModified /* = 0 */ )
//------------------------------------------------------------------------------
{
    m_pParentGrid = pPropTree->GetPropGrid();
    if( !wxPGIdIsOk( m_GridItemId ) )
    {
        const wxString elementName( GetDisplayName( flags ) );
        const TComponentType type = GetComponent().type();
        switch( type )
        {
        case ctPropInt:
        case ctPropInt64:
        case ctPropFloat:
            {
                const Property prop( GetComponent() );
                TComponentFlag componentFlags( prop.flags() );
                if( ( componentFlags & cfShouldBeDisplayedAsEnumeration ) || prop.hasDict() )
                {
                    if( componentFlags & cfAllowValueCombinations )
                    {
                        m_Type = _ctrlMultiChoiceSelector;
                    }
                    else
                    {
                        m_Type = _ctrlCombo;
                    }
                }
                else
                {
                    m_Type = _ctrlSpinner;
                }
            }
            break;
        case ctPropString:
            {
                wxString lowerCaseName( elementName.Lower() );
                if( lowerCaseName.Contains( wxString( wxT( "directory" ) ) ) )
                {
                    m_Type = _ctrlDirSelector;
                }
                else if( lowerCaseName.Contains( wxString( wxT( "filename" ) ) ) )
                {
                    m_Type = _ctrlFileSelector;
                }
                else if( GetComponent().flags() & cfContainsBinaryData )
                {
                    m_Type = _ctrlBinaryDataEditor;
                }
                else
                {
                    m_Type = _ctrlEdit;
                }
            }
            break;
        case ctPropPtr:
            m_Type = _ctrlEdit;
            break;
        default:
            break;
        }

        bool boIsSelector = GetComponent().selectedFeatureCount() > 0;
        switch( m_Type )
        {
        case _ctrlSpinner:
            if( ( type == ctPropInt ) || ( type == ctPropInt64 ) || ( type == ctPropFloat ) )
            {
                if( ( flags & dfSelectorGrouping ) && boIsSelector )
                {
#if wxPROPGRID_MINOR > 2
                    wxPropertyCategory* const prop = new wxPropertyCategory( GetDisplayName( flags ) );
#else
                    wxPGPropertyWithChildren* const prop = new wxParentPropertyClass( GetDisplayName( flags ), wxPG_LABEL );
#endif
                    // strictly speaking, prop could be NULL
                    m_GridItemId = m_pParentGrid->AppendIn( parentItem, prop );
                    if( m_GridItemId != prop )
                    {
                        wxASSERT( !"Invalid parenting" ); // no need to delete prop, wxPropertyGridState::PrepareToAddItem already did that
                    }
                }
                else
                {
                    m_GridItemId = m_pParentGrid->AppendIn( parentItem, NOW_NEW wxStringProperty( elementName, wxPG_LABEL ) );
                }
            }
            else
            {
                wxASSERT( !"invalid component type for spinner control" );
            }
            m_pParentGrid->SetPropertyEditor( m_GridItemId, wxPGCustomSpinCtrlEditor::Instance() );
            break;
        case _ctrlEdit:
            m_GridItemId = m_pParentGrid->AppendIn( parentItem, NOW_NEW wxLongStringProperty( elementName, wxPG_LABEL ) );
            break;
        case _ctrlCombo:
            {
                if( ( flags & dfSelectorGrouping ) && boIsSelector )
                {
#if wxPROPGRID_MINOR > 2
                    wxPropertyCategory* const prop = new wxPropertyCategory( GetDisplayName( flags ) );
#else
                    wxCustomPropertyClass* const prop = new wxCustomPropertyClass( GetDisplayName( flags ), wxPG_LABEL );
#endif
                    // strictly speaking, prop could be NULL
                    m_GridItemId = m_pParentGrid->AppendIn( parentItem, prop );
                    m_pParentGrid->SetPropertyEditor( m_GridItemId, wxPG_EDITOR( Choice ) );
                    if( m_GridItemId != prop )
                    {
                        wxASSERT( !"Invalid parenting" ); // no need to delete prop, wxPropertyGridState::PrepareToAddItem already did that
                    }
                }
                else
                {
                    wxPGChoices soc;
                    GetTransformedDict( soc );
                    m_GridItemId = m_pParentGrid->AppendIn( parentItem, NOW_NEW wxEnumProperty( elementName, wxPG_LABEL, soc, 240 ) );
                }
            }
            break;
        case _ctrlMultiChoiceSelector:
            {
                wxPGChoices soc;
                wxString emptyString;
                GetTransformedDict( soc, &emptyString );
                wxPGProperty* p = wxMultiChoiceProperty( elementName, wxPG_LABEL, soc.GetLabels() );
                dynamic_cast<wxMultiChoicePropertyClass*>( p )->SetEmptySelectionString( emptyString );
                m_GridItemId = m_pParentGrid->AppendIn( parentItem, p );
            }
            break;
        case _ctrlFileSelector:
            m_GridItemId = m_pParentGrid->AppendIn( parentItem, NOW_NEW wxFileProperty( elementName, wxPG_LABEL ) );
            break;
        case _ctrlDirSelector:
            m_GridItemId = m_pParentGrid->AppendIn( parentItem, NOW_NEW wxDirProperty( elementName, wxPG_LABEL, ::wxGetUserHome() ) );
            break;
        case _ctrlBinaryDataEditor:
            m_GridItemId = m_pParentGrid->AppendIn( parentItem, NOW_NEW wxBinaryDataProperty( elementName, wxPG_LABEL ) );
            m_pParentGrid->SetPropertyValidator( m_GridItemId, *wxBinaryDataPropertyClass::GetClassValidator() );
            break;
        default:
            break;
        }

        m_pParentGrid->SetPropertyClientData( m_GridItemId, this );
        if( pboModified )
        {
            *pboModified = true;
        }

        if( ( flags & dfSelectorGrouping ) && boIsSelector )
        {
            vector<mvIMPACT::acquire::Component> selectedFeatures;
            const vector<mvIMPACT::acquire::Component>::size_type cnt = GetComponent().selectedFeatures( selectedFeatures );
            for( vector<mvIMPACT::acquire::Component>::size_type i = 0; i < cnt; i++ )
            {
                pPropTree->CreateGridProperty( selectedFeatures[i], m_GridItemId );
            }
        }
    }
}

//------------------------------------------------------------------------------
void PropertyObject::UpdateLabel( EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const
//------------------------------------------------------------------------------
{
    ConvertedString label( GetDisplayName( flags ) );
    if( m_boVectorAsList )
    {
        label.Append( wxString::Format( ( ( flags & dfHexIndices ) ? wxT( "[0x%x]" ) : wxT( "[%d]" ) ), m_Index ) );
    }
    else
    {
        const int valCount = Property( GetComponent() ).valCount();
        if( valCount > 1 )
        {
            label.Append( wxString::Format( wxT( "[%d]" ), valCount ) );
        }
    }
    if( flags & dfDisplayDebugInfo )
    {
        AppendComponentInfo( label, actChangedCount, actAttrChangedCount );
    }
    UpdateLabelAndHelpString( flags, label );
    m_pParentGrid->SetPropertyTextColour( m_GridItemId, IsWriteable() ? ( GetComponent().isDefault() ? GlobalDataStorage::Instance()->GetPropGridColour( GlobalDataStorage::pgcIsDefaultValue ) : wxSystemSettings::GetColour ( wxSYS_COLOUR_WINDOWTEXT ) ) : wxSystemSettings::GetColour( wxSYS_COLOUR_GRAYTEXT ) );
}

//------------------------------------------------------------------------------
void PropertyObject::Update( const PropTree* /*pPropTree*/, EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const
//------------------------------------------------------------------------------
{
    const Property prop( GetComponent() );
    switch( GetType() )
    {
    case _ctrlEdit:
        {
            string str;
            const unsigned int valCnt = prop.valCount();
            if( ( valCnt > 1 ) && !m_boVectorAsList )
            {
                for( unsigned int i = 0; i < valCnt; i++ )
                {
                    str.append( prop.readS( i ) );
                    str.append( " " );
                }
            }
            else
            {
                str.append( prop.readS( m_Index ) );
            }
            m_pParentGrid->SetPropertyValueString( m_GridItemId, ConvertedString( str ) );
        }
        break;
    case _ctrlSpinner:
    case _ctrlDirSelector:
    case _ctrlFileSelector:
    case _ctrlBinaryDataEditor:
    case _ctrlMultiChoiceSelector:
        m_pParentGrid->SetPropertyValueString( m_GridItemId, GetCurrentValueAsString() );
        break;
    case _ctrlCombo:
        if( actAttrChangedCount != m_lastChangedCounterAttr )
        {
            wxPGChoices soc;
            GetTransformedDict( soc );
            m_pParentGrid->SetPropertyChoices( m_GridItemId, soc );
            m_pParentGrid->SetPropertyClientData( m_GridItemId, const_cast<PropertyObject*>( this ) );
        }
        m_pParentGrid->SetPropertyValueString( m_GridItemId, GetCurrentValueAsString() );
        break;
    default:
        break;
    }

    // read-only / read-write state
    if( !IsWriteable() )
    {
        m_pParentGrid->DisableProperty( m_GridItemId );
    }
    else if( !m_pParentGrid->IsPropertyEnabled( m_GridItemId ) )
    {
        m_pParentGrid->EnableProperty( m_GridItemId );
    }
    UpdateLabel( flags, actChangedCount, actAttrChangedCount );
}

//=============================================================================
//========================= VectorPropertyObject ==============================
//=============================================================================
//-----------------------------------------------------------------------------
VectorPropertyObject::VectorPropertyObject( HOBJ hObj )
    : PropData( hObj ), m_boExpanded( false ) {}
//-----------------------------------------------------------------------------

//-----------------------------------------------------------------------------
VectorPropertyObject::~VectorPropertyObject()
//-----------------------------------------------------------------------------
{
    size_t propCnt = static_cast<size_t>( m_VectorItems.size() );
    m_pParentGrid->Freeze();
    for( size_t i = 0; i < propCnt; i++ )
    {
        DeleteGridProperty( i );
    }
    m_pParentGrid->Thaw();
}

//-----------------------------------------------------------------------------
void VectorPropertyObject::DeleteGridProperty( size_t index )
//-----------------------------------------------------------------------------
{
    if( m_VectorItems.at( index ) )
    {
        wxPGId item = m_VectorItems[index]->GetGridItem();
        if( wxPGIdIsOk( item ) )
        {
#if wxPROPGRID_MINOR > 2
            m_pParentGrid->DeleteProperty( item );
#else
            m_pParentGrid->Delete( item );
#endif
        }
        delete m_VectorItems[index];
        m_VectorItems[index] = 0;
    }
}

//-----------------------------------------------------------------------------
void VectorPropertyObject::OnExpand()
//------------------------------------------------------------------------------
{
    if( wxPGIdIsOk( m_GridItemId ) && m_pParentGrid )
    {
        m_boExpanded = m_pParentGrid->IsPropertyExpanded( m_GridItemId );
    }
}

//------------------------------------------------------------------------------
PropertyObject* VectorPropertyObject::GetVectorItem( int index )
//------------------------------------------------------------------------------
{
    const int vSize = static_cast<int>( m_VectorItems.size() );
    for( int i = vSize; i <= index; i++ )
    {
        m_VectorItems.push_back( new PropertyObject( GetComponent(), i ) );
    }
    return m_VectorItems.at( index );
}

//------------------------------------------------------------------------------
void VectorPropertyObject::EnsureValidGridItem( const PropTree* pPropTree, wxPGId parentItem, EDisplayFlags flags, bool* pboModified /* = 0 */ )
//------------------------------------------------------------------------------
{
    if( !wxPGIdIsOk( m_GridItemId ) )
    {
        m_pParentGrid = pPropTree->GetPropGrid();
#if wxPROPGRID_MINOR > 2
        wxPropertyCategory* const prop = new wxPropertyCategory( GetDisplayName( flags ) );
#else
        wxPGPropertyWithChildren* const prop = new wxParentPropertyClass( GetDisplayName( flags ), wxPG_LABEL );
#endif
        // strictly speaking, prop could be NULL
        m_GridItemId = m_pParentGrid->AppendIn( parentItem, prop );
        if( m_GridItemId != prop )
        {
            wxASSERT( !"Invalid parenting" ); // no need to delete prop, wxPropertyGridState::PrepareToAddItem already did that
        }
        m_Type = _ctrlStatic;
        m_pParentGrid->SetPropertyClientData( m_GridItemId, this );
        m_pParentGrid->DisableProperty( m_GridItemId );
        if( pboModified )
        {
            *pboModified = true;
        }
    }

    const unsigned int actChangedCount = GetComponent().changedCounter();
    const unsigned int actAttrChangedCount = GetComponent().changedCounterAttr();
    if( ( m_lastChangedCounter != actChangedCount ) || ( m_lastChangedCounterAttr != actAttrChangedCount ) )
    {
        const Property prop( GetComponent() );
        const unsigned int valCount = prop.valCount();
        const unsigned int vSize = static_cast<unsigned int>( m_VectorItems.size() );
        if( valCount != vSize )
        {
            m_pParentGrid->Freeze();
        }
        for( unsigned int i = 0; i < prop.valCount(); i++ )
        {
            pPropTree->CreateGridProperty( GetComponent(), m_GridItemId, i );
        }
        if( valCount != vSize )
        {
            m_pParentGrid->Thaw();
        }
    }
}

//-----------------------------------------------------------------------------
void VectorPropertyObject::RemoveValue( unsigned int index )
//-----------------------------------------------------------------------------
{
    const Property prop( GetComponent() );
    const unsigned int vSize = static_cast<unsigned int>( m_VectorItems.size() );
    if( ( prop.valCount() > index ) && ( vSize > index ) && ( vSize > 1 ) )
    {
        prop.removeValue( index );
        DeleteGridProperty( index );
        m_VectorItems.erase( m_VectorItems.begin() + index );
    }
}

//------------------------------------------------------------------------------
void VectorPropertyObject::Resize( void )
//------------------------------------------------------------------------------
{
    const Property prop( GetComponent() );
    const unsigned int valCount = prop.valCount();
    const unsigned int vSize = static_cast<unsigned int>( m_VectorItems.size() );
    if( valCount < vSize )
    {
        m_pParentGrid->Freeze();
        for( unsigned int i = valCount; i < vSize; i++ )
        {
            wxPGId item = m_VectorItems[i]->GetGridItem();
            if( wxPGIdIsOk( item ) )
            {
#if wxPROPGRID_MINOR > 2
                m_pParentGrid->DeleteProperty( item );
#else
                m_pParentGrid->Delete( item );
#endif
            }
            delete m_VectorItems[i];
        }
        m_VectorItems.resize( valCount );
        m_pParentGrid->Thaw();
    }
    else if( valCount > vSize )
    {
        m_pParentGrid->Freeze();
        for( unsigned int i = vSize; i < valCount; i++ )
        {
            m_VectorItems.push_back( new PropertyObject( GetComponent(), static_cast<int>( i ) ) );
        }
        m_pParentGrid->Thaw();
    }
}

//-----------------------------------------------------------------------------
void VectorPropertyObject::Update( const PropTree* pPropTree, EDisplayFlags flags, unsigned int actChangedCount, unsigned int actAttrChangedCount ) const
//-----------------------------------------------------------------------------
{
    const Property prop( GetComponent() );
    const unsigned int valCnt = prop.valCount();
    unsigned int vSize = static_cast<unsigned int>( m_VectorItems.size() );
    ConvertedString label( GetDisplayName( flags ) );
    label.Append( wxString::Format( wxT( "[%d]" ), valCnt ) );
    if( flags & dfDisplayDebugInfo )
    {
        AppendComponentInfo( label, actChangedCount, actAttrChangedCount );
    }
    m_pParentGrid->SetPropertyLabel( m_GridItemId, label );
    wxString val( wxT( "[" ) );
    val.Append( ConvertedString( prop.readSArray( "", ", " ) ) );
    val.Append( wxT( "]" ) );
    wxPGIdToPtr( m_GridItemId )->SetValueFromString( val );
    while( valCnt > vSize )
    {
        pPropTree->CreateGridProperty( GetComponent(), m_GridItemId, vSize++ );
    }
    m_pParentGrid->SetPropertyTextColour( m_GridItemId, prop.isDefault() ? GlobalDataStorage::Instance()->GetPropGridColour( GlobalDataStorage::pgcIsDefaultValue ) : wxSystemSettings::GetColour( wxSYS_COLOUR_WINDOWTEXT ) );
}
