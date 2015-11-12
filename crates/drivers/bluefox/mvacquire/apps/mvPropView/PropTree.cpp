//-----------------------------------------------------------------------------
#include "DataConversion.h"
#include "PropData.h"
#include "PropTree.h"
//------------------------------------------------------------------------------

using namespace std;
using namespace mvIMPACT::acquire;

//------------------------------------------------------------------------------
PropTree::PropTree( HOBJ hObj, const char* pTitle, wxPropertyGrid* pPropGrid, EDisplayFlags flags ) :
    m_rootList( hObj ), m_pPropGrid( pPropGrid ), m_TopLevelProp( wxNullProperty ),
    m_Title( pTitle ? pTitle : string() ), m_flags( flags )
//------------------------------------------------------------------------------
{
}

//------------------------------------------------------------------------------
PropTree::~PropTree( void )
//------------------------------------------------------------------------------
{
    m_pPropGrid->Freeze();
    CompToDataMap::iterator it = m_CompToDataMap.begin();
    CompToDataMap::iterator itEnd = m_CompToDataMap.end();
    while( it != itEnd )
    {
        RemoveFromGlobalNameToFeatureMap( it->second );
        delete it->second;
        ++it;
    }
    Delete();
    m_pPropGrid->Thaw();
}

//-----------------------------------------------------------------------------
wxString PropTree::BuildFullFeatureName( Component comp ) const
//-----------------------------------------------------------------------------
{
    wxString fullFeatureName( ConvertedString( comp.name() ) );
    ComponentIterator cit( comp );
    while( cit.isValid() && ( cit.hObj() != m_rootList ) )
    {
        cit = cit.parent();
        if( cit.isValid() )
        {
            fullFeatureName.Prepend( wxString::Format( wxT( "%s/" ), ConvertedString( cit.name() ).c_str() ) );
        }
    }
    return fullFeatureName;
}

//------------------------------------------------------------------------------
wxPGId PropTree::CreateGridProperty( HOBJ hObj, wxPGId parentProp, int index /* = -1 */, bool* pboModified /* = 0 */, const char* pTitle /* = 0 */ ) const
//------------------------------------------------------------------------------
{
    if( pboModified )
    {
        *pboModified = false;
    }

    // check if item is already present
    PropData* pPropData = GetPropData( hObj );
    if( !pPropData )
    {
        const Component comp( hObj );
        bool boCompToDataMap = true;
        switch( comp.type() )
        {
        case ctList:
            pPropData = new ListObject( hObj, pTitle );
            break;
        case ctMeth:
            pPropData = new MethodObject( hObj );
            break;
        case ctPropInt:
        case ctPropInt64:
        case ctPropFloat:
        case ctPropString:
        case ctPropPtr:
            {
                boCompToDataMap = index == -1;
                const bool boPropVector = ( index == -1 ) && ( comp.flags() & cfShouldBeDisplayedAsList );
                if( boPropVector )
                {
                    pPropData = new VectorPropertyObject( hObj );
                }
                else
                {
                    pPropData = new PropertyObject( hObj, index );
                }
            }
            break;
        default:
            wxASSERT( !"unrecognized component type" );
            break;
        }
        if( boCompToDataMap )
        {
            const wxString featureFullName( BuildFullFeatureName( comp ) );
            m_CompToDataMap.insert( make_pair( hObj, pPropData ) );
            m_DataToCompMap.insert( make_pair( pPropData, hObj ) );
            GlobalDataStorage::Instance()->nameToFeatureMap_.insert( make_pair( featureFullName, pPropData ) );
            GlobalDataStorage::Instance()->featureToNameMap_.insert( make_pair( pPropData, featureFullName ) );
            pPropData->SetFeatureFullName( featureFullName );
        }
    }
    else if( index != -1 )
    {
        VectorPropertyObject* const pPropVec = dynamic_cast<VectorPropertyObject*>( pPropData );
        wxASSERT( pPropVec != 0 );
        pPropVec->Resize();
        pPropData = pPropVec->GetVectorItem( index );
    }
    wxASSERT( pPropData != 0 );
    pPropData->EnsureValidGridItem( this, parentProp, m_flags, pboModified );
    //if( !m_pPropGrid->IsPropertySelected( pPropData->GetGridItem() ) )
    //{
    pPropData->UpdateGridItem( this, m_flags, pboModified );
    //}
    return pPropData->GetGridItem();
}

//-----------------------------------------------------------------------------
void PropTree::Delete( void )
//-----------------------------------------------------------------------------
{
    if( wxPGIdIsOk( m_TopLevelProp ) )
    {
#if wxPROPGRID_MINOR > 2
        m_pPropGrid->DeleteProperty( m_TopLevelProp );
#else
        m_pPropGrid->Delete( m_TopLevelProp );
#endif
        m_TopLevelProp = wxNullProperty;
    }
}

//------------------------------------------------------------------------------
unsigned int PropTree::Draw( bool boForceRedraw )
//------------------------------------------------------------------------------
{
    if( m_rootList.isValid() )
    {
        bool boModified = false;
        /// \todo freeze / thaw mechanism causes flickering editors (focus problem)
        //m_pPropGrid->Freeze();
        m_TopLevelProp = CreateGridProperty( m_rootList, 0, -1, &boModified, m_Title.empty() ? 0 : m_Title.c_str() );

        if( wxPGIdIsOk( m_TopLevelProp ) )
        {
            // do NOT change the order of the next two calls as then deleted entries won't be
            // removed from the property grid correctly...
            // As an alternative the '&& pPropData->HasChanged()' could be removed from 'UpdateGridPropsRecursively2'
            UpdateGridPropsRecursively2( m_pPropGrid->GetFirstChild( m_TopLevelProp ) );
            UpdateGridPropsRecursively( ComponentIterator( m_rootList.firstChild() ), m_TopLevelProp, boForceRedraw );
        }
        else
        {
            wxASSERT( !"Invalid mid level wxPGId" );
        }
        //m_pPropGrid->Thaw();
        //m_pPropGrid->CalculateYs( NULL, -1 );
        //m_pPropGrid->Refresh();
    }
    else
    {
        wxASSERT( !"Invalid ComponentIterator" );
    }

    return GetChangedCounter();
}

//------------------------------------------------------------------------------
PropData* PropTree::GetPropData( HOBJ hObj ) const
//------------------------------------------------------------------------------
{
    CompToDataMap::const_iterator it = m_CompToDataMap.find( hObj );
    return ( it != m_CompToDataMap.end() ) ? it->second : 0;
}

//-----------------------------------------------------------------------------
void PropTree::RemoveFromGlobalNameToFeatureMap( PropData* pPropData ) const
//-----------------------------------------------------------------------------
{
    FeatureToNameMap::iterator itFeatureToName = GlobalDataStorage::Instance()->featureToNameMap_.find( pPropData );
    wxASSERT( ( itFeatureToName != GlobalDataStorage::Instance()->featureToNameMap_.end() ) && "Inconsistent internal maps" );
    NameToFeatureMap::iterator itNameToFeature = GlobalDataStorage::Instance()->nameToFeatureMap_.find( itFeatureToName->second );
    wxASSERT( ( itNameToFeature != GlobalDataStorage::Instance()->nameToFeatureMap_.end() ) && "Inconsistent internal maps" );
    if( itFeatureToName != GlobalDataStorage::Instance()->featureToNameMap_.end() )
    {
        GlobalDataStorage::Instance()->featureToNameMap_.erase( itFeatureToName );
    }
    if( itNameToFeature != GlobalDataStorage::Instance()->nameToFeatureMap_.end() )
    {
        GlobalDataStorage::Instance()->nameToFeatureMap_.erase( itNameToFeature );
    }
}

//------------------------------------------------------------------------------
/// \brief Updates and/or creates grid properties
void PropTree::UpdateGridPropsRecursively( ComponentIterator iter, wxPGId listRoot, bool boForceRedraw ) const
//------------------------------------------------------------------------------
{
    // wxPGIdIsOk(listRoot) is checked before the call

    while( iter.isValid() )
    {
        bool boModified;
        wxPGId gridProp = CreateGridProperty( iter, listRoot, -1, &boModified );
        if( iter.isList() && wxPGIdIsOk( gridProp ) && ( boModified || boForceRedraw ) )
        {
            PropData* const pPropData = reinterpret_cast<PropData*>( m_pPropGrid->GetPropertyClientData( gridProp ) );
            if( pPropData )
            {
                // this sublist needs to be updated as there have been changes in it...
                UpdateGridPropsRecursively( iter.firstChild(), gridProp, boForceRedraw );
            }
        }
        ++iter;
    }
}

//-----------------------------------------------------------------------------
/// \brief Removes references and grid items that became invalid.
void PropTree::UpdateGridPropsRecursively2( wxPGId iter, bool boForceDelete /* = false */ ) const
//-----------------------------------------------------------------------------
{
    while( wxPGIdIsOk( iter ) )
    {
        PropData* const pPropData = reinterpret_cast<PropData*>( m_pPropGrid->GetPropertyClientData( iter ) );
        wxPGId toDelete = wxNullProperty;
        if( pPropData->GetComponent().isValid() )
        {
            const wxString featureFullName( BuildFullFeatureName( pPropData->GetComponent() ) );
            if( featureFullName != pPropData->GetFeatureFullName() )
            {
                // the handle is valid (again!) but has been assigned to a different feature
                // this might happen when one or more feature lists inside the driver are deleted and
                // then recreated with new handles
                boForceDelete = true;
            }
        }
        if( boForceDelete || !pPropData->GetComponent().isValid() )
        {
            if( ( m_pPropGrid->GetChildrenCount( iter ) > 0 ) && !dynamic_cast<VectorPropertyObject*>( pPropData ) )
            {
                // if we have to delete an entry we also have to delete all children of this entry
                UpdateGridPropsRecursively2( m_pPropGrid->GetFirstChild( iter ), true );
            }
            DataToCompMap::iterator it = m_DataToCompMap.find( pPropData );
            if( it != m_DataToCompMap.end() )
            {
                CompToDataMap::iterator itCompToData = m_CompToDataMap.find( pPropData->GetComponent().hObj() );
                wxASSERT( ( itCompToData != m_CompToDataMap.end() ) && "Inconsistent internal maps" );
                if( itCompToData != m_CompToDataMap.end() )
                {
                    m_CompToDataMap.erase( itCompToData );
                }
                m_DataToCompMap.erase( it );
                RemoveFromGlobalNameToFeatureMap( pPropData );
            }
            delete pPropData;
            toDelete = iter;
        }
        else if( pPropData->GetComponent().isList() && pPropData->HasChanged() )
        {
            UpdateGridPropsRecursively2( m_pPropGrid->GetFirstChild( iter ) );
        }

#if wxPROPGRID_MINOR > 2
        const wxPGPropertyWithChildren* const parent = iter->GetParent();
        const size_t next_ind = iter->GetArrIndex() + 1;
        iter = next_ind < parent->GetCount() ? parent->Item( next_ind ) : wxNullProperty;
#else
        iter = ( ( wxPropertyContainerMethods* ) m_pPropGrid )->GetNextSibling( iter );
#endif
        if( wxPGIdIsOk( toDelete ) )
        {
#if wxPROPGRID_MINOR > 2
            m_pPropGrid->DeleteProperty( toDelete );
#else
            m_pPropGrid->Delete( toDelete );
#endif
        }
    }
}
